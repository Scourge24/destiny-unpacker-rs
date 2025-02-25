mod structs;
mod utils;
use utils::*;
use structs::*;
extern crate getopts;
use getopts::Options;
use std::{thread, fs, env, io::SeekFrom, io::BufWriter, io::BufReader, io::prelude::*, fs::File};
use openssl::{cipher::Cipher, cipher_ctx::CipherCtx};

const BLOCK_SIZE: u32 = 262144;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {}", program);
    print!("{}", opts.usage(&brief));
}

fn main()
{
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.reqopt("p", "", "Packages Path", "PATH");
    opts.reqopt("i", "", "Package ID", "ID");
    opts.optopt("o", "", "Output Path", "PATH");
    opts.optopt("n", "nonaudio", "Does NOT skip non-audio related files", "");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { println!("{}",f); print_usage(&program, opts); return; }
    };

    let pkgspath = matches.opt_str("p").unwrap();
    let pkgid = matches.opt_str("i").unwrap();
    let mut _output_path_base:String = String::new();
    if matches.opt_present("o") {
        _output_path_base = matches.opt_str("o").unwrap();
    }
    else {
        _output_path_base = format!("{}/output/{}", env::current_dir().unwrap().display(), pkgid);
    }

    let mut skip_non_audio:bool = true;
    if matches.opt_present("n") {
        skip_non_audio = false;
    }

    let mut package = Package::new(pkgspath, pkgid);
    read_header(&mut package);
    modify_nonce(&mut package);
    read_entry_table(&mut package);
    read_block_table(&mut package);
    extract_files(package, _output_path_base, skip_non_audio);
    println!("Done extracting.");
}

pub fn read_header(package: &mut structs::Package) -> bool
{
    let mut u16buffer = [0; 2];
    let mut u32buffer = [0; 4];
    let file = File::open(package.package_path.clone()).expect("Error reading file");
    let mut reader = BufReader::new(file);
    let mut header = Header::new();
    reader.seek(SeekFrom::Start(0x10)).expect("Error seeking file");
    

    reader.read_exact(&mut u16buffer).expect("Error reading file");
    header.pkgid = le_u16(&u16buffer);

    reader.seek(SeekFrom::Start(0x30)).expect("Error seeking");
    reader.read_exact(&mut u16buffer).expect("Error reading file");
    header.patchid = le_u16(&u16buffer);

    reader.seek(SeekFrom::Start(0x44)).expect("Error seeking");
    reader.read_exact(&mut u32buffer).expect("Error reading file");
    header.entry_table_offset = le_u32(&u32buffer);
    
    reader.seek(SeekFrom::Start(0x60)).expect("Error seeking");
    reader.read_exact(&mut u32buffer).expect("Error reading file");
    header.entry_table_size = le_u32(&u32buffer);
    
    reader.seek(SeekFrom::Start(0x68)).expect("Error seeking");
    reader.read_exact(&mut u32buffer).expect("Error reading file");
    header.block_table_size = le_u32(&u32buffer);
    reader.read_exact(&mut u32buffer).expect("Error reading file");
    header.block_table_offset = le_u32(&u32buffer);

    reader.seek(SeekFrom::Start(0xB8)).expect("Error seeking");
    reader.read_exact(&mut u32buffer).expect("Error reading file");
    header.hash64_table_size = le_u32(&u32buffer);
    reader.read_exact(&mut u32buffer).expect("Error reading file");
    header.hash64_table_offset = le_u32(&u32buffer);
    header.hash64_table_offset += 64;

    reader.seek(SeekFrom::Start(0)).expect("Error seeking");

    package.header = header;

    true
}

pub fn read_entry_table(package: &mut Package) -> bool
{
    let file = File::open(package.package_path.clone()).expect("Error reading file");
    let mut reader = BufReader::new(file);
    let a = package.header.entry_table_offset+package.header.entry_table_size*16;
    for i in (package.header.entry_table_offset.to_owned()..a).step_by(16)
    {
        let mut entry: Entry = Entry::new();

        let mut u32buffer = [0; 4];
        reader.seek(SeekFrom::Start(i.into())).expect("Error seeking");
        reader.read_exact(&mut u32buffer).expect("Error reading file");
        let entrya:u32 = be_u32(&u32buffer);
        entry.reference = format!("{:08x}", entrya);
        
        reader.read_exact(&mut u32buffer).expect("Error reading file");
        let entryb:u32 = le_u32(&u32buffer);
        entry.numtype = ((entryb >> 9) & 0x7F) as u8;
        entry.numsubtype = ((entryb >> 6) & 0x7) as u8;

        reader.read_exact(&mut u32buffer).expect("Error reading file");
        let entryc:u32 = le_u32(&u32buffer);
        
        entry.startingblock = entryc & 16383;
        entry.startingblockoffset = ((entryc >> 14) & 16383) << 4;

        reader.read_exact(&mut u32buffer).expect("Error reading file");
        let entryd:u32 = le_u32(&u32buffer);

        entry.filesize = (entryd & 0x03FFFFFF) << 4 | (entryc >> 28) & 0xF;

        package.entries.push(entry);
    }
    reader.seek(SeekFrom::Start(0)).expect("Error seeking");
    package.entries.remove(0);
    
    true
}

pub fn read_block_table(package: &mut structs::Package) -> bool
{
    let file = File::open(package.package_path.clone()).expect("Error reading file");
    let mut reader = BufReader::new(file);
    let a = package.header.block_table_offset+package.header.block_table_size*48;
    for b in (package.header.block_table_offset..a).step_by(48)
    {
        let mut block: Block = Block::new();
        let mut u32buffer = [0; 4];
        let mut u16buffer = [0; 2];
        let mut gcmtag_buffer = [0; 16];
        reader.seek(SeekFrom::Start(b.into())).expect("Error seeking");
        reader.read_exact(&mut u32buffer).expect("Error reading file");
        block.offset = le_u32(&u32buffer);
        reader.read_exact(&mut u32buffer).expect("Error reading file");
        block.size = le_u32(&u32buffer);
        
        reader.read_exact(&mut u16buffer).expect("Error reading file");
        block.patchid = le_u16(&u16buffer);
        
        reader.read_exact(&mut u16buffer).expect("Error reading file");
        block.bitflag = le_u16(&u16buffer);

        reader.seek(SeekFrom::Current(0x20)).expect("Error seeking");
        reader.read_exact(&mut gcmtag_buffer).expect("Error reading file");
        block.gcmtag = gcmtag_buffer;
        package.blocks.push(block);
    }
    reader.seek(SeekFrom::Start(0)).expect("Error seeking");
    package.blocks.remove(0);

    true
}

fn modify_nonce(package: &mut structs::Package)
{
    package.nonce[0] ^= (package.header.pkgid >> 8) as u8;
    package.nonce[11] ^= package.header.pkgid as u8;
}

fn extract_files(package: structs::Package, output_path_base: String, skip_non_audio: bool)
{
    let mut pkg_patch_stream_paths: Vec<String> = Vec::new();
    for i in 0..=package.header.patchid
    {
        let a = i as u8 + 48;
        let pkg_patch_path = package.package_path.clone();
        let mut b:String = pkg_patch_path.to_string();
        b.remove(b.len()-5);
        b.insert(pkg_patch_path.len()-5, a as char);
        pkg_patch_stream_paths.push(b.to_string());
    }
    let thread = thread::spawn(move || {
        for i in 0..package.entries.len()
        {
            let entry = &package.entries[i];
            
            if skip_non_audio && !(entry.numtype == 26 && (entry.numsubtype == 6 || entry.numsubtype == 7)) {
                continue;
            }

            let mut cur_block_id = entry.startingblock;
            let mut block_count:u32 = libm::floorf((entry.startingblockoffset as f32 + entry.filesize as f32 - 1.0_f32) / BLOCK_SIZE as f32) as u32;
            if entry.filesize == 0
            {
                block_count = 0;
            }
            let last_block_id = cur_block_id + block_count;
            let mut file_buffer = vec![0u8; entry.filesize as usize];
            let mut current_buffer_offset = 0;
            while cur_block_id <= last_block_id
            {
                let current_block = &package.blocks[cur_block_id as usize];
                let file = File::open(&pkg_patch_stream_paths[current_block.patchid as usize]).expect("Error reading file");
                let mut reader = BufReader::new(file);
                reader.seek(SeekFrom::Start(current_block.offset as u64)).expect("Error seeking");
                let mut block_buffer = vec![0; current_block.size as usize];
                let result = reader.read(&mut block_buffer).expect("Error reading file");
                if result != current_block.size as usize
                {
                    println!("Error reading file");
                }
                let mut _decrypt_buffer:Vec<u8> = vec![0u8; current_block.size as usize];
                let mut _decomp_buffer:Vec<u8> = vec![0u8; BLOCK_SIZE as usize];
                if current_block.bitflag & 0x2 != 0
                {
                    _decrypt_buffer = decrypt_block(&package, current_block, block_buffer);
                }
                else
                {
                    
                    _decrypt_buffer = block_buffer
                }
                if current_block.bitflag & 0x1 != 0
                {
                    _decomp_buffer = decompress_block(current_block, &mut _decrypt_buffer);
                }
                else
                {
                    _decomp_buffer = _decrypt_buffer;
                }
                if cur_block_id == entry.startingblock
                {
                    let mut _cpy_size = 0;

                    if cur_block_id == last_block_id
                    {
                        _cpy_size = entry.filesize;
                    }
                    else
                    {
                        _cpy_size = BLOCK_SIZE - entry.startingblockoffset;
                    }
                    file_buffer[0.._cpy_size as usize].copy_from_slice(&_decomp_buffer[entry.startingblockoffset as usize..entry.startingblockoffset as usize + _cpy_size as usize]);

                    current_buffer_offset += _cpy_size as usize;
                }
                else if cur_block_id == last_block_id
                {
                    file_buffer[current_buffer_offset as usize..]
                    .copy_from_slice(&_decomp_buffer[..(entry.filesize - current_buffer_offset as u32) as usize]);
                }
                else
                {
                    file_buffer[current_buffer_offset as usize..(current_buffer_offset + BLOCK_SIZE as usize) as usize].copy_from_slice(&_decomp_buffer[0..BLOCK_SIZE as usize]);
                    current_buffer_offset += BLOCK_SIZE as usize;
                }
                reader.seek(SeekFrom::Start(0)).expect("Error seeking");
                cur_block_id +=1;
                _decomp_buffer.clear();
            }
            let mut cus_out = output_path_base.clone();
            let mut _file_name:String = String::new();
            let mut _ext = "";
            if entry.numtype == 26 && entry.numsubtype == 7
            {
                _ext = "wem";
                cus_out += "\\wem";
                _file_name = entry.reference.to_uppercase();
            }
            else if entry.numtype == 26 && entry.numsubtype == 6
            {
                _ext = "bnk";
                cus_out.push_str("/bnk"); 
                _file_name = format!("{}-{:04x}", package.package_id, i);
            }
            else
            {
                _ext = "bin";
                cus_out.push_str(format!("/unknown/{}/", entry.reference.to_uppercase()).as_str());
                _file_name = get_hash_from_file(format!("{}-{:04x}", package.package_id, i));
            }          
            fs::create_dir_all(&cus_out).expect("Error creating directory");
            let mut stream = BufWriter::new(File::create(format!("{}/{}.{}", cus_out, _file_name, _ext)).expect("Error creating file"));
            stream.write_all(&file_buffer).expect("Error writing file");
            stream.flush().unwrap();
            file_buffer.clear();
        }
    });
    thread.join().unwrap();
}

fn decrypt_block(package: &structs::Package, block: &structs::Block, mut block_buffer: Vec<u8>) -> Vec<u8>
{
    let mut decrypt_buffer:Vec<u8> = vec![];
    let alt_key = &block.bitflag & 4 != 0;
    let mut _key = &[0u8; 16];
    if alt_key
    {
        _key = &package.aes_alt_key;
    }
    else
    {
        _key = &package.aes_key;
    };
    let cipher = Cipher::aes_128_gcm();
    let mut ctx = CipherCtx::new().unwrap();
    ctx.decrypt_init(Some(cipher), Some(_key), Some(&package.nonce)).unwrap();
    ctx.set_tag(&block.gcmtag).unwrap();
    ctx.cipher_update_vec(&block_buffer, &mut decrypt_buffer).unwrap();
    ctx.cipher_final_vec(&mut decrypt_buffer).expect_err("Failed finalizing decrypter");

    block_buffer.clear();

    decrypt_buffer
}

#[allow(non_snake_case)]
fn decompress_block(block: &structs::Block, decrypt_buffer: &mut Vec<u8>, ) -> Vec<u8>
{
    unsafe {
        let mut decomp_buffer = [0u8; BLOCK_SIZE as usize];
        let lib = libloading::Library::new("oo2core_9_win64.dll").expect("Failed to load Oodle.");
        let OodleLZ_Decompress: libloading::Symbol<extern "C" fn(compressed_bytes: &u8, size_of_compressed_bytes:i64, decompressed_bytes: *mut u8, size_of_decompressed_bytes:i64,
            a:u32, b:u32, c:u32, d:u32, e:u32, f:u32, g:u32, h:u32, i:u32, threadModule:u32) -> i64> = lib.get(b"OodleLZ_Decompress").expect("Failed to load OodleLZ_Decompress function.");
        let _result:i64 = OodleLZ_Decompress(&decrypt_buffer[0], block.size as i64, &mut decomp_buffer[0], BLOCK_SIZE as i64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3);
        decrypt_buffer.clear();
        
        decomp_buffer.to_vec()
    }
}