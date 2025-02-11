use std::collections::HashMap;
use std::path::PathBuf;

use radix_engine::engine::Substate;
use radix_engine::ledger::*;
use radix_engine::types::*;
use rocksdb::{DBWithThreadMode, Direction, IteratorMode, SingleThreaded, DB};

pub struct RadixEngineDB {
    db: DBWithThreadMode<SingleThreaded>,
}

impl RadixEngineDB {
    pub fn new(root: PathBuf) -> Self {
        let db = DB::open_default(root.as_path()).unwrap();
        Self { db }
    }

    pub fn with_bootstrap(root: PathBuf) -> Self {
        let substate_store = Self::new(root);
        bootstrap(substate_store)
    }

    pub fn list_packages(&self) -> Vec<PackageAddress> {
        let start = &scrypto_encode(&SubstateId::Package(PackageAddress::Normal([0; 26])));
        let end = &scrypto_encode(&SubstateId::Package(PackageAddress::Normal([255; 26])));
        let substate_ids: Vec<SubstateId> = self.list_items(start, end);
        substate_ids
            .into_iter()
            .map(|id| {
                if let SubstateId::Package(package_address) = id {
                    package_address
                } else {
                    panic!("Expected a package substate id.")
                }
            })
            .collect()
    }

    fn list_components_helper(
        &self,
        start: ComponentAddress,
        end: ComponentAddress,
    ) -> Vec<ComponentAddress> {
        let start = &scrypto_encode(&SubstateId::ComponentState(start));
        let end = &scrypto_encode(&SubstateId::ComponentState(end));
        let substate_ids: Vec<SubstateId> = self.list_items(start, end);
        substate_ids
            .into_iter()
            .map(|id| {
                if let SubstateId::ComponentState(component_address) = id {
                    component_address
                } else {
                    panic!("Expected a component substate id.")
                }
            })
            .collect()
    }

    pub fn list_components(&self) -> Vec<ComponentAddress> {
        let mut addresses = Vec::new();
        addresses.extend(self.list_components_helper(
            ComponentAddress::System([0u8; 26]),
            ComponentAddress::System([255u8; 26]),
        ));
        addresses.extend(self.list_components_helper(
            ComponentAddress::Account([0u8; 26]),
            ComponentAddress::Account([255u8; 26]),
        ));
        addresses.extend(self.list_components_helper(
            ComponentAddress::Normal([0u8; 26]),
            ComponentAddress::Normal([255u8; 26]),
        ));
        addresses
    }

    pub fn list_resource_managers(&self) -> Vec<ResourceAddress> {
        let start = &scrypto_encode(&SubstateId::ResourceManager(ResourceAddress::Normal(
            [0; 26],
        )));
        let end = &scrypto_encode(&SubstateId::ResourceManager(ResourceAddress::Normal(
            [255; 26],
        )));
        let substate_ids: Vec<SubstateId> = self.list_items(start, end);
        substate_ids
            .into_iter()
            .map(|id| {
                if let SubstateId::ResourceManager(resource_address) = id {
                    resource_address
                } else {
                    panic!("Expected a resource substate id.")
                }
            })
            .collect()
    }

    fn list_items<T: Decode>(&self, start: &[u8], inclusive_end: &[u8]) -> Vec<T> {
        let mut iter = self
            .db
            .iterator(IteratorMode::From(start, Direction::Forward));
        let mut items = Vec::new();
        while let Some(kv) = iter.next() {
            let (key, _value) = kv.unwrap();
            if key.as_ref() > inclusive_end {
                break;
            }
            if key.len() == start.len() {
                items.push(scrypto_decode(key.as_ref()).unwrap());
            }
        }
        items
    }

    fn read(&self, substate_id: &SubstateId) -> Option<Vec<u8>> {
        // TODO: Use get_pinned
        self.db.get(scrypto_encode(substate_id)).unwrap()
    }

    fn write(&self, substate_id: SubstateId, value: Vec<u8>) {
        self.db.put(scrypto_encode(&substate_id), value).unwrap();
    }
}

impl QueryableSubstateStore for RadixEngineDB {
    fn get_kv_store_entries(&self, kv_store_id: &KeyValueStoreId) -> HashMap<Vec<u8>, Substate> {
        let unit = scrypto_encode(&());
        let id = scrypto_encode(&SubstateId::KeyValueStoreEntry(
            kv_store_id.clone(),
            scrypto_encode(&unit),
        ));

        let mut iter = self
            .db
            .iterator(IteratorMode::From(&id, Direction::Forward));
        let mut items = HashMap::new();
        while let Some(kv) = iter.next() {
            let (key, value) = kv.unwrap();
            let substate: OutputValue = scrypto_decode(&value.to_vec()).unwrap();
            let substate_id: SubstateId = scrypto_decode(&key).unwrap();
            if let SubstateId::KeyValueStoreEntry(id, key) = substate_id {
                if id == *kv_store_id {
                    items.insert(key, substate.substate)
                } else {
                    break;
                }
            } else {
                break;
            };
        }
        items
    }
}

// Implement this as an enum for now to prevent clashes with Substates
// TODO: Have a better key prefixing strategy
#[derive(Debug, Clone, TypeId, Encode, Decode)]
pub enum Root {
    Root(SubstateId),
}

impl ReadableSubstateStore for RadixEngineDB {
    fn get_substate(&self, substate_id: &SubstateId) -> Option<OutputValue> {
        self.read(substate_id).map(|b| scrypto_decode(&b).unwrap())
    }

    fn is_root(&self, substate_id: &SubstateId) -> bool {
        self.db
            .get(scrypto_encode(&Root::Root(substate_id.clone())))
            .unwrap()
            .is_some()
    }
}

impl WriteableSubstateStore for RadixEngineDB {
    fn put_substate(&mut self, substate_id: SubstateId, substate: OutputValue) {
        self.write(substate_id, scrypto_encode(&substate));
    }

    fn set_root(&mut self, substate_id: SubstateId) {
        self.db
            .put(scrypto_encode(&Root::Root(substate_id)), vec![])
            .unwrap();
    }
}
