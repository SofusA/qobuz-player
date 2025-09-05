use std::sync::Arc;

use tokio::sync::RwLock;

#[derive(Debug)]
pub struct ReadOnly<T>(Arc<RwLock<T>>);

impl<T> Clone for ReadOnly<T> {
    fn clone(&self) -> Self {
        ReadOnly(self.0.clone())
    }
}

impl<T> ReadOnly<T> {
    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.0.read().await
    }
}

impl<T> From<Arc<RwLock<T>>> for ReadOnly<T> {
    fn from(arc: Arc<RwLock<T>>) -> Self {
        ReadOnly(arc)
    }
}
