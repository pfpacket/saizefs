use rs9p::srv::{Fid, Filesystem};
use rs9p::*;
use std::collections::HashMap;
use std::error::Error;
use std::path::{Component, Path, PathBuf};

macro_rules! errno {
    ($kind:ident) => {
        rs9p::Error::No(rs9p::errno::$kind);
    };
}

fn make_qid(typ: rs9p::QidType, path: u64) -> rs9p::Qid {
    rs9p::Qid {
        typ,
        version: 0,
        path,
    }
}

fn make_stat(is_dir: bool) -> rs9p::Stat {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let sys_time = rs9p::Time {
        sec: duration.as_secs(),
        nsec: duration.subsec_nanos() as u64,
    };

    rs9p::Stat {
        mode: if is_dir {
            libc::S_IFDIR | libc::S_IXUSR | libc::S_IXGRP | libc::S_IXOTH
        } else {
            libc::S_IFREG
        } | libc::S_IRUSR
            | libc::S_IRGRP
            | libc::S_IROTH,
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
        nlink: 1,
        rdev: 0,
        size: 0,
        blksize: 1,
        blocks: 1,
        atime: sys_time,
        mtime: sys_time,
        ctime: sys_time,
    }
}

fn get_dirent(entry: &Entry, offset: u64) -> DirEntry {
    rs9p::DirEntry {
        qid: entry.qid,
        offset,
        typ: 0,
        name: entry.name.clone(),
    }
}

#[derive(Debug)]
struct Entry {
    name: String,
    qid: rs9p::Qid,
    stat: rs9p::Stat,
    data: String,
    entries: HashMap<String, Entry>,
}

impl Entry {
    fn new(name: String, typ: rs9p::QidType, inode: u64, data: String) -> Entry {
        Entry {
            name,
            qid: make_qid(typ, inode),
            stat: make_stat(typ == rs9p::QidType::DIR),
            data,
            entries: HashMap::new(),
        }
    }
}

struct Saizefs {
    root: Entry,
}

impl Saizefs {
    fn new<P: AsRef<Path>>(db_path: P) -> rusqlite::Result<Saizefs> {
        let menudb = rusqlite::Connection::open(db_path)?;

        let mut dishes = menudb.prepare("SELECT * FROM menu")?;
        let dishes = dishes.query_map(rusqlite::NO_PARAMS, |row| {
            Ok(vec![
                row.get::<usize, String>(1)?,                 // name
                row.get::<usize, u32>(0)?.to_string() + "\n", // id
                row.get::<usize, String>(2)? + "\n",          // category
                row.get::<usize, String>(3)? + "\n",          // type
                row.get::<usize, u32>(4)?.to_string() + "\n", // price
                row.get::<usize, u32>(5)?.to_string() + "\n", // calorie
                row.get::<usize, f64>(6)?.to_string() + "\n", // salt
            ])
        })?;

        let mut root = Entry::new("/".to_owned(), rs9p::QidType::DIR, 0, "".to_owned());

        let mut inode = 1;
        for dish in dishes.filter_map(|d| d.ok()) {
            let mut dish_entry =
                Entry::new(dish[0].clone(), rs9p::QidType::DIR, inode, "".to_owned());
            inode += 1;

            for (i, item_name) in ["name", "id", "category", "type", "price", "calorie", "salt"]
                .iter()
                .enumerate()
                .skip(1)
            {
                dish_entry.entries.insert(
                    item_name.to_string(),
                    Entry::new(
                        item_name.to_string(),
                        rs9p::QidType::FILE,
                        inode,
                        dish[i].clone(),
                    ),
                );
                inode += 1;
            }

            root.entries.insert(dish[0].clone(), dish_entry);
        }

        Ok(Saizefs { root })
    }

    fn get_node<P: AsRef<Path>>(&self, p: P) -> Option<&Entry> {
        let path = p.as_ref();
        if path == Path::new("/") {
            return Some(&self.root);
        }

        let mut components = path.components();
        if components.next() != Some(Component::RootDir) {
            return None;
        }

        let dir = match components.next() {
            Some(Component::Normal(dir)) => dir.to_string_lossy().into_owned(),
            _ => return None,
        };

        let file = match components.next() {
            Some(Component::Normal(file)) => file.to_string_lossy().into_owned(),
            None => return self.root.entries.get(&dir),
            _ => return None,
        };

        if components.next() == None {
            return self
                .root
                .entries
                .get(&dir)
                .and_then(|d| d.entries.get(&file));
        }

        None
    }
}

struct SaizefsFid {
    path: PathBuf,
    already_read: bool,
}

impl Filesystem for Saizefs {
    type Fid = SaizefsFid;

    fn rattach(
        &mut self,
        fid: &mut Fid<Self::Fid>,
        _afid: Option<&mut Fid<Self::Fid>>,
        _uname: &str,
        _aname: &str,
        _n_uname: u32,
    ) -> Result<Fcall> {
        fid.aux = Some(SaizefsFid {
            path: PathBuf::from("/"),
            already_read: false,
        });

        Ok(Fcall::Rattach { qid: self.root.qid })
    }

    fn rwalk(
        &mut self,
        fid: &mut Fid<Self::Fid>,
        newfid: &mut Fid<Self::Fid>,
        wnames: &[String],
    ) -> Result<Fcall> {
        let mut wqids = Vec::new();
        let mut path = fid.aux().path.clone();

        for (i, name) in wnames.iter().enumerate() {
            path.push(name);

            let qid = match self.get_node(&path) {
                Some(entry) => entry.qid,
                None => {
                    if i == 0 {
                        return Err(errno!(ENOENT));
                    } else {
                        break;
                    }
                }
            };

            wqids.push(qid);
        }

        newfid.aux = Some(SaizefsFid {
            path,
            already_read: false,
        });

        Ok(Fcall::Rwalk { wqids })
    }

    fn rgetattr(&mut self, fid: &mut Fid<Self::Fid>, req_mask: GetattrMask) -> Result<Fcall> {
        let entry = self.get_node(&fid.aux().path).ok_or(errno!(EINVAL))?;

        Ok(Fcall::Rgetattr {
            valid: req_mask,
            qid: entry.qid,
            stat: entry.stat,
        })
    }

    fn rreaddir(&mut self, fid: &mut Fid<Self::Fid>, off: u64, count: u32) -> Result<Fcall> {
        let mut dirents = DirEntryData::new();
        let entry = self.get_node(&fid.aux().path).ok_or(errno!(EINVAL))?;

        for (i, (_, entry)) in entry.entries.iter().enumerate().skip(off as usize) {
            let dirent = get_dirent(entry, 2 + i as u64);
            if dirents.size() + dirent.size() > count {
                break;
            }
            dirents.push(dirent);
        }

        Ok(Fcall::Rreaddir { data: dirents })
    }

    fn rlopen(&mut self, fid: &mut Fid<Self::Fid>, _flags: u32) -> Result<Fcall> {
        let entry = self.get_node(&fid.aux().path).ok_or(errno!(EINVAL))?;

        Ok(Fcall::Rlopen {
            qid: entry.qid,
            iounit: 0,
        })
    }

    fn rread(&mut self, fid: &mut Fid<Self::Fid>, _offset: u64, _count: u32) -> Result<Fcall> {
        let mut fid = fid.aux_mut();
        let entry = self.get_node(&fid.path).ok_or(errno!(EINVAL))?;

        let buf = if fid.already_read {
            Data(Vec::new())
        } else {
            fid.already_read = true;
            Data(entry.data.clone().into_bytes())
        };

        Ok(Fcall::Rread { data: buf })
    }

    fn rclunk(&mut self, _: &mut Fid<Self::Fid>) -> Result<Fcall> {
        Ok(Fcall::Rclunk)
    }
}

fn main() -> std::result::Result<(), Box<dyn Error + Send + Sync>> {
    env_logger::init();

    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} proto!address!port", args[0]);
        println!("  where: proto = tcp | unix");
        return Ok(());
    }

    let addr = &args[1];
    println!("[*] Ready to accept clients: {}", addr);

    rs9p::srv_spawn(Saizefs::new("./saizeriya.db")?, addr).map_err(From::from)
}

#[test]
fn saizefs_basic_test() {
    let fs = Saizefs::new("./saizeriya.db").unwrap();
    println!("{:?}", fs.get_node("/").unwrap().name);
    println!("{:?}", fs.get_node("/小エビのサラダ").unwrap().name);
    println!(
        "{:?}",
        fs.get_node("/小エビのサラダ")
            .unwrap()
            .entries
            .iter()
            .map(|(_, e)| e.name.clone())
            .collect::<Vec<String>>()
    );
    println!(
        "{:?}",
        fs.get_node("/小エビのサラダ")
            .unwrap()
            .entries
            .iter()
            .map(|(_, e)| e.data.clone())
            .collect::<Vec<String>>()
    );
    println!("{:?}", fs.get_node("/イタリアンサラダ/price").unwrap().data);
}
