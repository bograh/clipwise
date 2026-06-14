use crate::clipboard::ClipboardItem;
use std::error::Error;
use std::path::PathBuf;

pub struct Storage {
    db: sled::Db,
}

impl Storage {
    pub fn open() -> Result<Self, Box<dyn Error>> {
        let home = std::env::var("HOME").map_err(|e| format!("HOME not set: {}", e))?;
        let path = PathBuf::from(home).join(".local/share/clipwise/db");
        std::fs::create_dir_all(path.parent().unwrap())?;
        let db = sled::open(&path)?;
        Ok(Storage { db })
    }

    pub fn save_all(&self, items: &[ClipboardItem]) -> Result<(), Box<dyn Error>> {
        let tree = self.db.open_tree("items")?;
        tree.clear()?;

        for item in items {
            let value = serde_json::to_vec(item)?;
            tree.insert(item.id.as_bytes(), value)?;
        }

        let order: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        tree.insert(b"__order__", serde_json::to_vec(&order)?)?;

        self.db.flush()?;
        Ok(())
    }

    // Upsert a single item and refresh the stored order in one flush.
    // Avoids the clear+rewrite of save_all when only one item changed.
    pub fn save_item_and_order(
        &self,
        item: &ClipboardItem,
        items: &[ClipboardItem],
    ) -> Result<(), Box<dyn Error>> {
        let tree = self.db.open_tree("items")?;
        tree.insert(item.id.as_bytes(), serde_json::to_vec(item)?)?;
        let order: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        tree.insert(b"__order__", serde_json::to_vec(&order)?)?;
        self.db.flush()?;
        Ok(())
    }

    // Remove a single item from sled and refresh the stored order in one flush.
    pub fn delete_item_and_order(
        &self,
        id: &str,
        items: &[ClipboardItem],
    ) -> Result<(), Box<dyn Error>> {
        let tree = self.db.open_tree("items")?;
        tree.remove(id.as_bytes())?;
        let order: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        tree.insert(b"__order__", serde_json::to_vec(&order)?)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn load_all(&self) -> Result<Vec<ClipboardItem>, Box<dyn Error>> {
        let tree = self.db.open_tree("items")?;

        let order_bytes = match tree.get(b"__order__")? {
            Some(b) => b,
            None => return Ok(vec![]),
        };

        let order: Vec<String> = serde_json::from_slice(&order_bytes)?;
        let mut items = Vec::new();

        for id in &order {
            if let Some(bytes) = tree.get(id.as_bytes())? {
                if let Ok(item) = serde_json::from_slice::<ClipboardItem>(&bytes) {
                    items.push(item);
                }
            }
        }

        Ok(items)
    }
}
