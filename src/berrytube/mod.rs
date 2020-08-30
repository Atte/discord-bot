use crate::{Result, discord::InitialActivityKey, util::ellipsis_string};
use serenity::{
    client::bridge::gateway::ShardManager,
    prelude::{Mutex, RwLock, TypeMap},
    model::gateway::Activity
};
use std::sync::Arc;

mod sse;

pub struct Berrytube {
    shard_manager: Arc<Mutex<ShardManager>>,
    data: Arc<RwLock<TypeMap>>,
}

impl Berrytube {
    pub fn new(shard_manager: Arc<Mutex<ShardManager>>, data: Arc<RwLock<TypeMap>>) -> Self {
        Self {
            shard_manager,
            data,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        Ok(())
    }

    async fn set_title(&self, title: String) {
        let truncated = ellipsis_string(&title, 128);
        {
            let mut data = self.data.write().await;
            data.insert::<InitialActivityKey>(title);
        }
        {
            let shard_manager = self.shard_manager.lock().await;
            for runner in shard_manager.runners.lock().await.values() {
                runner.runner_tx.set_activity(Some(Activity::playing(&truncated)));
            }
        }
    }
}
