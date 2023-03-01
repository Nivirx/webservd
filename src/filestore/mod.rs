use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, RwLock, Mutex};
use std::thread;

use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{ Receiver, channel};
use std::time::Duration;

type FileEntryGuard<T> = Arc<RwLock<T>>;

#[derive(Debug)]
enum FileEntryError {
    NeedsUpdate,
    EmptyFile,
    NoFileEntry
}
#[derive(Clone, Debug)]
pub struct FileEntry {
    file: FileEntryGuard<std::fs::File>,
    contents: FileEntryGuard<Option<String>>,
    last_accessed: FileEntryGuard<Option<std::time::SystemTime>>
}

impl FileEntry {

    pub fn new(path: &str) -> FileEntry {
        FileEntry {
            file: Arc::new(RwLock::new(std::fs::File::open(path).unwrap())),
            contents: Arc::new(RwLock::new(None)),
            last_accessed: Arc::new(RwLock::new(None))
        }
    }

    fn get(&self) -> Result<String, FileEntryError>  {
        let access_time = *Arc::clone(&self.last_accessed).read().unwrap();
        match access_time {
            Some(_) => {
                //file has been updated at somepoint
                log::warn!("Cache hit");
                match &*Arc::clone(&self.contents).read().unwrap() {
                    Some(s) => Ok(s.clone()),
                    None => Err(FileEntryError::EmptyFile)
                }
            },
            None => {
                log::warn!("Cache miss");
                Err(FileEntryError::NeedsUpdate)
            }
        }
    }

    fn update(&mut self) {
        let f = Arc::clone(&self.file);
        let mut f = f.write().unwrap();

        let contents =  Arc::clone(&self.contents);
        match contents.write().unwrap().as_mut() {
            Some(_s) => {
                //f.read_to_string(&mut s);
                log::warn!("got Some(_) while updating an entry");
            },
            None => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).unwrap();
                self.contents = Arc::new(RwLock::new(Some(buf)));
            }
        };

        *Arc::clone(&self.last_accessed).write().unwrap() = Some(std::time::SystemTime::now());
    }
}

type StoreGuard<T> = Arc<Mutex<T>>;
#[allow(dead_code)]
pub struct FileCache {
    store: StoreGuard< HashMap<String, FileEntryGuard<FileEntry>> >,
    notify_dir: String,
    notify_watcher: RecommendedWatcher,
    notify_thread: std::thread::JoinHandle<()>
}

impl FileCache {
    pub fn new(dir_watch: &str) -> FileCache {
        let (tx, rx) = channel();



        let mut fc = FileCache {
            store: Arc::new(Mutex::new(HashMap::new())),
            notify_dir: dir_watch.to_string(),
            notify_watcher: notify::Watcher::new(tx, Duration::from_secs(5)).unwrap(),
            notify_thread: thread::Builder::new().name("notify-thread".to_string())
                                .spawn(move || { FileCache::notify_loop(rx) }).unwrap(),
        };
        fc.notify_watcher.watch(&fc.notify_dir, RecursiveMode::Recursive).unwrap();
        log::info!("started notify watcher on {}", &fc.notify_dir);

        fc
    }

    pub fn open(&self, path: &str) {
        let fe = FileEntry::new(&path.to_string());
        let store = Arc::clone(&self.store);
        let mut store = store.lock().unwrap();
        store.entry(path.to_string()).or_insert(Arc::new(RwLock::new(fe)));
    }
    pub fn read(&self, path: &str) -> String {
        match self.lookup(path) {
            Ok(s) => return s,
            Err(e) => {
                match e {
                    FileEntryError::NeedsUpdate => {
                        let store = Arc::clone(&self.store);
                        let store = store.lock().unwrap();

                       let fe = Arc::clone(&store.get(path).unwrap());
                       let mut fe = fe.write().unwrap();

                        fe.update();
                        fe.get().unwrap()

                    },
                    FileEntryError::EmptyFile => return "".to_string(),
                    FileEntryError::NoFileEntry => return "".to_string(),
                }
            }

        }
    }
    fn lookup(&self, path: &str) -> Result<String, FileEntryError> {
        let store = Arc::clone(&self.store);
        let store = store.lock().unwrap();

        let result = match store.get(path) {
            Some(fe) => {
                //this is mutable because the FileEntry.get() may update the contents
                Some(fe)
            },
            //TODO: this shouldn't just return an empty string if there isn't a k,v pair
            None => {
                None
            }
        };

        //log::debug!("hashmap contents at end of read {:#?}", store);

        match result {
            Some(fe) => {
                let r = Arc::clone(&fe).read().unwrap().get();
                match r {
                    Ok(s) => {
                        return Ok(s.clone())
                    },
                    Err(e) => {
                        return Err(e)
                    }
                }
            },
            None => {
                return Err(FileEntryError::NoFileEntry)
            }
        };
    }

    fn invalidate_entry(&self, path: &str) -> Option<(String, FileEntryGuard<FileEntry>)>  {
        let store = Arc::clone(&self.store);
        let mut store = store.lock().unwrap();
        let value = store.remove_entry(&path.to_string());

        return value
    }
    fn notify_loop(rx: Receiver<DebouncedEvent>) {
        log::info!("notify loop starting on thread-{:?}", std::thread::current());
        loop {
            match rx.recv() {
                Ok(event) => { 
                    log::info!("{:?}", &event);

                    match event {
                        DebouncedEvent::NoticeWrite(_) => {
                            // do nothing, this is sent when a file is being updated
                        },
                        DebouncedEvent::NoticeRemove(_) => {
                            // do nothing, this is sent when a file is being removed
                        },
                        DebouncedEvent::Create(p) => {
                            if let Some(t) = crate::FILECACHE.invalidate_entry(p.to_str().unwrap()) {
                                log::info!("invalidated {} from FileCache", &t.0)
                            }
                        },
                        DebouncedEvent::Write(p) => {
                            if let Some(t) = crate::FILECACHE.invalidate_entry(p.to_str().unwrap()) {
                                log::info!("invalidated {} from FileCache", &t.0)
                            }
                        },
                        DebouncedEvent::Chmod(_) => log::debug!("received a Chmod event on watched dir, but we don't do anything!"),
                        DebouncedEvent::Remove(p) => {
                            if let Some(t) = crate::FILECACHE.invalidate_entry(p.to_str().unwrap()) {
                                log::info!("invalidated {} from FileCache", &t.0)
                            }
                        },
                        DebouncedEvent::Rename(p, _) => {
                            if let Some(t) = crate::FILECACHE.invalidate_entry(p.to_str().unwrap()) {
                                log::info!("invalidated {} from FileCache", &t.0)
                            }
                        },
                        DebouncedEvent::Rescan => log::debug!("received a Rescan event on watched dir, but we don't do anything!"),
                        DebouncedEvent::Error(_, _) => log::debug!("received a Error event on watched dir, but we don't do anything!"),
                    }
                },
                Err(e) => log::info!("watch error: {:?}", e),
            }
        }
    }
}