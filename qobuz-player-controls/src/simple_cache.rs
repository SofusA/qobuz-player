use time::Duration;
use tokio::{sync::RwLock, time::Instant};

#[derive(Debug)]
pub(crate) struct SimpleCache<T> {
    value: RwLock<Option<T>>,
    ttl: Duration,
    created: RwLock<Option<Instant>>,
}

impl<T> SimpleCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            value: RwLock::new(None),
            ttl,
            created: RwLock::new(None),
        }
    }

    pub async fn get(&self) -> Option<T>
    where
        T: Clone,
    {
        if self.valid().await {
            self.value.read().await.clone()
        } else {
            None
        }
    }

    pub async fn set(&self, value: T) {
        let mut val_lock = self.value.write().await;
        let mut time_lock = self.created.write().await;
        *val_lock = Some(value);
        *time_lock = Some(Instant::now());
    }

    pub async fn clear(&self) {
        let mut val_lock = self.value.write().await;
        let mut time_lock = self.created.write().await;
        *val_lock = None;
        *time_lock = None;
    }

    async fn valid(&self) -> bool {
        let time_lock = self.created.read().await;
        match *time_lock {
            Some(created) => created.elapsed() < self.ttl,
            None => false,
        }
    }
}
