// file: src/database/rocksdb.rs
use rocksdb::{DB, Options, ColumnFamilyDescriptor};
use tokio::sync::RwLock;

pub struct BlockchainDB {
    db: Arc<DB>,
    block_cf: ColumnFamily,
    tx_cf: ColumnFamily,
    utxo_cf: ColumnFamily,
    chainstate_cf: ColumnFamily,
}

impl BlockchainDB {
    pub async fn new(path: &str) -> Result<Self, DatabaseError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new("blocks", Options::default()),
            ColumnFamilyDescriptor::new("transactions", Options::default()),
            ColumnFamilyDescriptor::new("utxo", Options::default()),
            ColumnFamilyDescriptor::new("chainstate", Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        Ok(Self {
            db: Arc::new(db),
            // ... initialize column families
        })
    }

    /// Fast UTXO lookup (O(1) instead of O(n))
    pub async fn get_utxo(&self, outpoint: &OutPoint) -> Result<Option<UTXO>, DatabaseError> {
        let key = outpoint.to_db_key();
        self.db.get_cf(&self.utxo_cf, key).await
    }
}