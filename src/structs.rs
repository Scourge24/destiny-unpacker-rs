use std::path::Path;
use std::process;
use std::sync::{Once};

#[derive(Clone)]
pub struct Entry {
    pub reference: String,
    pub numtype: u8,
    pub numsubtype: u8,
    pub startingblock: u32,
    pub startingblockoffset: u32,
    pub filesize: u32
}

#[derive(Clone)]
pub struct Header {
    pub pkgid: u16,
    pub patchid: u16,
    pub entry_table_offset: u32,
    pub entry_table_size: u32,
    pub block_table_offset: u32,
    pub block_table_size: u32,
    pub hash64_table_offset: u32,
    pub hash64_table_size: u32,
}

#[derive(Clone)]
pub struct Block
{
	pub id: u32,
	pub offset: u32,
    pub size: u32,
    pub patchid: u16,
    pub bitflag: u16,
    pub gcmtag: [u8; 16],
}

pub struct Package {
    pub header: Header,
    pub packages_path: String,
    pub entries: Vec<Entry>,
    pub package_path: String,
    pub package_filename : String,
    pub package_id: String,
    pub nonce: [u8; 12],
    pub blocks: Vec<Block>,
    pub aes_key: [u8; 16],
    pub aes_alt_key: [u8; 16],
}

#[derive(Clone)]
pub struct ExtrOpts {
    pub hexid:bool,
    pub skip_non_audio:bool,
    pub wavconv:bool,
    pub output_path:String
}

impl Package {
    pub fn new(pkgspath:&str, pkgid:&str) -> Package {
            let mut _exists:bool=true;
            let packages_path = pkgspath.to_owned();
            let package_id = pkgid.to_owned();
            _exists = Path::new(&packages_path).exists();
            if !_exists {
                println!("Packages Path does not exist");
                process::exit(1);
            }
            let package_path = get_latest_patch_id_path(&packages_path, &package_id);
            let package_filename = package_path[package_path.rfind("/").unwrap() + 1..package_path.len() - 6].to_owned();//[package_path.rfind("/") ..package_path.len() - 4]
            let pkgp = package_path;
            Package {
                header: Header::new(),
                nonce: [0x84, 0xEA, 0x11, 0xC0, 0xAC, 0xAB, 0xFA, 0x20, 0x33, 0x11, 0x26, 0x99],
                blocks: vec![Block::new()],
                packages_path,
                package_filename,
                package_id,
                package_path: pkgp,
                entries: vec![Entry::new()],
                aes_key: [0xD6, 0x2A, 0xB2, 0xC1, 0x0C, 0xC0, 0x1B, 0xC5, 0x35, 0xDB, 0x7B, 0x86, 0x55, 0xC7, 0xDC, 0x3B],
                aes_alt_key: [0x3A, 0x4A, 0x5D, 0x36, 0x73, 0xA6, 0x60, 0x58, 0x7E, 0x63, 0xE6, 0x76, 0xE4, 0x08, 0x92, 0xB5],
            }
    }
}

fn get_latest_patch_id_path(packages_path: &str, package_id: &str) -> String {
    static INIT: Once = Once::new();
    static mut PATHS: Vec<String> = Vec::new();
    INIT.call_once(||
    {
        for entry in std::fs::read_dir(packages_path).unwrap() {
            unsafe {
                PATHS.push(entry.unwrap().path().display().to_string());
            }
        }
    });

    let paths = unsafe
    {
        &PATHS
    };

    let mut latest_patch_id:u16 = u16::MIN;
    let mut package_name:String = String::new();
    for path in paths {
        //println!("{}",path);     
        if path.contains(package_id) {
            //println!("Match: {}",path);
            let patch_str = &path[path.len()-5..path.len()-4];
            //println!("{}",&path[path.len()-5..]);
            //println!("{}",patch_str);
            let patch_id:u16 = patch_str.parse::<u16>().unwrap();
            if patch_id > latest_patch_id || latest_patch_id == 0 {
                latest_patch_id = patch_id;
                let path2 = path.replace('\\', "/");
                package_name = path2[path2.rfind('/').unwrap() + 1..path2.len()-6].to_string();
                //println!("{package_name}");
                /*
                let pos = package_name.rfind('/');
                let val = package_name.len()-pos.unwrap();
                package_name = package_name[pos.unwrap()..].to_string();
                package_name = package_name[..val].to_string();
                */
                //println!("Latest Patch Id: {}", latest_patch_id);
                //println!("Latest Patch Path: {}", package_name);
            }
        }
    }
    //println!("{packages_path}/{package_name}_{latest_patch_id}.pkg");
    return format!("{}/{}_{}.pkg", packages_path, package_name, &latest_patch_id.to_string());
}

impl Header {
    pub fn new() -> Header {
        Header {
            pkgid: 0,
            patchid: 0,
            entry_table_offset: 0,
            entry_table_size: 0,
            block_table_offset: 0,
            block_table_size: 0,
            hash64_table_offset: 0,
            hash64_table_size: 0,
        }
    }
}
impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Entry {
    pub fn new() -> Entry {
        Entry {
            reference: String::new() ,
            numtype: 0,
            numsubtype: 0,
            startingblock: 0,
            startingblockoffset: 0,
            filesize: 0,
        }
    }
}
impl Default for Entry {
    fn default() -> Self {
        Self::new()
    }
}

impl Block {
    pub fn new() -> Block {
        Block {
            id: 0,
            offset: 0,
            size: 0,
            patchid: 0,
            bitflag: 0,
            gcmtag: [0; 16],
        }
    }
}
impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtrOpts {
    pub fn new() -> ExtrOpts {
        ExtrOpts {
            hexid: false,
            skip_non_audio: true,
            wavconv: false,
            output_path: String::new()
        }
    }
}
impl Default for ExtrOpts {
    fn default() -> Self {
        Self::new()
    }
}