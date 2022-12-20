use getopts::Options;
use std::env;
use std::io::{Seek, Read, Write, SeekFrom};
use std::path::Path;
use std::fs::File;
use std::vec::Vec;
use elf;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

const NIN_LOGO_DEFAULT: [u8; 156] = [
    0x24,0xFF,0xAE,0x51,0x69,0x9A,0xA2,0x21,0x3D,0x84,0x82,0x0A,0x84,0xE4,0x09,0xAD,
    0x11,0x24,0x8B,0x98,0xC0,0x81,0x7F,0x21,0xA3,0x52,0xBE,0x19,0x93,0x09,0xCE,0x20,
    0x10,0x46,0x4A,0x4A,0xF8,0x27,0x31,0xEC,0x58,0xC7,0xE8,0x33,0x82,0xE3,0xCE,0xBF,
    0x85,0xF4,0xDF,0x94,0xCE,0x4B,0x09,0xC1,0x94,0x56,0x8A,0xC0,0x13,0x72,0xA7,0xFC,
    0x9F,0x84,0x4D,0x73,0xA3,0xCA,0x9A,0x61,0x58,0x97,0xA3,0x27,0xFC,0x03,0x98,0x76,
    0x23,0x1D,0xC7,0x61,0x03,0x04,0xAE,0x56,0xBF,0x38,0x84,0x00,0x40,0xA7,0x0E,0xFD,
    0xFF,0x52,0xFE,0x03,0x6F,0x95,0x30,0xF1,0x97,0xFB,0xC0,0x85,0x60,0xD6,0x80,0x25,
    0xA9,0x63,0xBE,0x03,0x01,0x4E,0x38,0xE2,0xF9,0xA2,0x34,0xFF,0xBB,0x3E,0x03,0x44,
    0x78,0x00,0x90,0xCB,0x88,0x11,0x3A,0x94,0x65,0xC0,0x7C,0x63,0x87,0xF0,0x3C,0xAF,
    0xD6,0x25,0xE4,0x8B,0x38,0x0A,0xAC,0x72,0x21,0xD4,0xF8,0x07
];
const NIN_LOGO_CRC_DEFAULT: u16 = 0xCF56;

// The "older/faster MROM" setting described on GBATEK.
// These values are probably ignored by flashcarts / emulators, but whatever.
const ROM_CTRL_1_DEFAULT: u32 = 0x00586000;
const ROM_CTRL_2_DEFAULT: u32 = 0x001808F8;
const ROM_CTRL_3_DEFAULT: u16 = 0x051E;

const CRC_16_TABLE: [u16; 256] = [
    0x0000, 0xC0C1, 0xC181, 0x0140, 0xC301, 0x03C0, 0x0280, 0xC241,
    0xC601, 0x06C0, 0x0780, 0xC741, 0x0500, 0xC5C1, 0xC481, 0x0440,
    0xCC01, 0x0CC0, 0x0D80, 0xCD41, 0x0F00, 0xCFC1, 0xCE81, 0x0E40,
    0x0A00, 0xCAC1, 0xCB81, 0x0B40, 0xC901, 0x09C0, 0x0880, 0xC841,
    0xD801, 0x18C0, 0x1980, 0xD941, 0x1B00, 0xDBC1, 0xDA81, 0x1A40,
    0x1E00, 0xDEC1, 0xDF81, 0x1F40, 0xDD01, 0x1DC0, 0x1C80, 0xDC41,
    0x1400, 0xD4C1, 0xD581, 0x1540, 0xD701, 0x17C0, 0x1680, 0xD641,
    0xD201, 0x12C0, 0x1380, 0xD341, 0x1100, 0xD1C1, 0xD081, 0x1040,
    0xF001, 0x30C0, 0x3180, 0xF141, 0x3300, 0xF3C1, 0xF281, 0x3240,
    0x3600, 0xF6C1, 0xF781, 0x3740, 0xF501, 0x35C0, 0x3480, 0xF441,
    0x3C00, 0xFCC1, 0xFD81, 0x3D40, 0xFF01, 0x3FC0, 0x3E80, 0xFE41,
    0xFA01, 0x3AC0, 0x3B80, 0xFB41, 0x3900, 0xF9C1, 0xF881, 0x3840,
    0x2800, 0xE8C1, 0xE981, 0x2940, 0xEB01, 0x2BC0, 0x2A80, 0xEA41,
    0xEE01, 0x2EC0, 0x2F80, 0xEF41, 0x2D00, 0xEDC1, 0xEC81, 0x2C40,
    0xE401, 0x24C0, 0x2580, 0xE541, 0x2700, 0xE7C1, 0xE681, 0x2640,
    0x2200, 0xE2C1, 0xE381, 0x2340, 0xE101, 0x21C0, 0x2080, 0xE041,
    0xA001, 0x60C0, 0x6180, 0xA141, 0x6300, 0xA3C1, 0xA281, 0x6240,
    0x6600, 0xA6C1, 0xA781, 0x6740, 0xA501, 0x65C0, 0x6480, 0xA441,
    0x6C00, 0xACC1, 0xAD81, 0x6D40, 0xAF01, 0x6FC0, 0x6E80, 0xAE41,
    0xAA01, 0x6AC0, 0x6B80, 0xAB41, 0x6900, 0xA9C1, 0xA881, 0x6840,
    0x7800, 0xB8C1, 0xB981, 0x7940, 0xBB01, 0x7BC0, 0x7A80, 0xBA41,
    0xBE01, 0x7EC0, 0x7F80, 0xBF41, 0x7D00, 0xBDC1, 0xBC81, 0x7C40,
    0xB401, 0x74C0, 0x7580, 0xB541, 0x7700, 0xB7C1, 0xB681, 0x7640,
    0x7200, 0xB2C1, 0xB381, 0x7340, 0xB101, 0x71C0, 0x7080, 0xB041,
    0x5000, 0x90C1, 0x9181, 0x5140, 0x9301, 0x53C0, 0x5280, 0x9241,
    0x9601, 0x56C0, 0x5780, 0x9741, 0x5500, 0x95C1, 0x9481, 0x5440,
    0x9C01, 0x5CC0, 0x5D80, 0x9D41, 0x5F00, 0x9FC1, 0x9E81, 0x5E40,
    0x5A00, 0x9AC1, 0x9B81, 0x5B40, 0x9901, 0x59C0, 0x5880, 0x9841,
    0x8801, 0x48C0, 0x4980, 0x8941, 0x4B00, 0x8BC1, 0x8A81, 0x4A40,
    0x4E00, 0x8EC1, 0x8F81, 0x4F40, 0x8D01, 0x4DC0, 0x4C80, 0x8C41,
    0x4400, 0x84C1, 0x8581, 0x4540, 0x8701, 0x47C0, 0x4680, 0x8641,
    0x8201, 0x42C0, 0x4380, 0x8341, 0x4100, 0x81C1, 0x8081, 0x4040 
];

// # https://problemkaputt.de/gbatek.htm#dscartridgeheader
#[repr(C, packed)]
struct Header {
    game_title: [u8; 12],       // (Uppercase ASCII, padded with 00h)
    game_code: [u8; 4],         // (Uppercase ASCII, NTR-<code>)        (0=homebrew)
    maker_code: [u8; 2],        // (Uppercase ASCII, eg. "01"=Nintendo) (0=homebrew)
    unit_code: u8,              // (00h=NDS, 02h=NDS+DSi, 03h=DSi) (bit1=DSi)
    encryption_seed_select: u8, // (00..07h, usually 00h)
    cart_capacity: u8,          // (Chipsize = 128KB SHL nn) (eg. 7 = 16MB)
    reserved1: [u8; 7],         // 0 filled
    reserved2: u8,              // 0 filled                    (except, used on DSi)
    region: u8,                 // (00h=Normal, 80h=China, 40h=Korea) (other on DSi)
    rom_version: u8,            // revision number of the ROM. usually 0 for commercial games
    autostart: u8,              // (Bit2: Skip "Press Button" after Health and Safety)
    arm9_rom_offset: u32,       // (4000h and up, align 1000h)
    arm9_entry_addr: u32,       // (2000000h..23BFE00h)
    arm9_ram_addr: u32,         // (2000000h..23BFE00h)
    arm9_size: u32,             // (max 3BFE00h) (3839.5KB)
    arm7_rom_offset: u32,       // (8000h and up)
    arm7_entry_addr: u32,       // (2000000h..23BFE00h, or 37F8000h..3807E00h)
    arm7_ram_addr: u32,         // (2000000h..23BFE00h, or 37F8000h..3807E00h)
    arm7_size: u32,             // (max 3BFE00h) (3839.5KB)
    fnt_offset: u32,            // File Name Table offset
    fnt_size: u32,              // File Name Table size
    fat_offset: u32,            // File Allocation Table offset
    fat_size: u32,              // File Allocation Table size
    file_arm9_overlay_ofs: u32, // used for nintendo's compiler, or something? not needed for homebrew
    file_arm9_overlay_size: u32,// "
    file_arm7_overlay_ofs: u32, // "
    file_arm7_overlay_size: u32,// "
    rom_ctrl1: u32,             // Sets ROMCTRL for normal commands
    rom_ctrl2: u32,             // Sets ROMCTRL for KEY1 commands
    icon_title_offset: u32,     // (0=None) (8000h and up)
    secure_area_checksum: u16,  // CRC-16 of [[arm9RomOffset]..00007FFFh]
    rom_ctrl3: u16,             // Secure Area Delay (in 131kHz units) (051Eh=10ms or 0D7Eh=26ms)
    arm9_auto_ld_list_hook: u32,// (?) endaddr of auto-load functions
    arm7_auto_ld_list_hook: u32,// "
    secure_area_disable: u64,   // (by encrypted "NmMdOnly") (usually zero)
    total_rom_size: u32,        // (remaining/unused bytes usually FFh-padded)
    rom_header_size: u32,       // 4000h
    reserved3: u32,             // Unknown, some rom_offset, or zero? (DSi: slightly different)
    reserved4: u64,             // (zero filled; except, [88h..93h] used on DSi)
    nand_rom_end: u16,          // \ in 20000h-byte units (DSi: 80000h-byte)
    nand_rw_start: u16,         // / usually both same address (0=None)
    reserved5: [u8; 24],        // Reserved (zero filled)
    reserved6: [u8; 16],        // Reserved (zero filled; or "DoNotZeroFillMem"=unlaunch fastboot)
    nin_logo: [u8; 156],        // Nintendo Logo (compressed bitmap, same as in GBA Headers)
    nin_logo_checksum: u16,     // Nintendo Logo Checksum, CRC-16 of [0C0h-15Bh], fixed CF56h
    header_checksum: u16,       // CRC-16 of [000h-15Dh]
    dbg_rom_offset: u32,        // (0=none) (8000h and up)       only if debug
    dbg_size: u32,              // (0=none) (max 3BFE00h)        version with
    dbg_ram_addr: u32,          // (0=none) (2400000h..27BFE00h) SIO and 8MB
    // Rest of header is 0 filled
    // (up to 0x200 sortof? but seems like it must be empty until 0x4000)
}

impl Default for Header {
    fn default() -> Self {
        Header {
            game_title: *b"HOMEBREW\0\0\0\0", // default from devkitarm
            game_code: *b"####",              // default from devkitarm
            maker_code: [0, 0],
            unit_code: 0,                     // NDS
            encryption_seed_select: 0,
            cart_capacity: 0,
            reserved1: [0; 7],
            reserved2: 0,
            region: 0,                        // "Normal" region (non China or Korea)
            rom_version: 0,
            autostart: 0,
            arm9_rom_offset: 0,
            arm9_entry_addr: 0,
            arm9_ram_addr: 0,
            arm9_size: 0,
            arm7_rom_offset: 0,
            arm7_entry_addr: 0,
            arm7_ram_addr: 0,
            arm7_size: 0,
            fnt_offset: 0,
            fnt_size: 0,
            fat_offset: 0,
            fat_size: 0,
            file_arm9_overlay_ofs: 0,
            file_arm9_overlay_size: 0,
            file_arm7_overlay_ofs: 0,
            file_arm7_overlay_size: 0,
            rom_ctrl1: ROM_CTRL_1_DEFAULT,
            rom_ctrl2: ROM_CTRL_2_DEFAULT,
            icon_title_offset: 0,
            secure_area_checksum: 0,
            rom_ctrl3: ROM_CTRL_3_DEFAULT,
            arm9_auto_ld_list_hook: 0,
            arm7_auto_ld_list_hook: 0,
            secure_area_disable: 0,
            total_rom_size: 0,
            rom_header_size: 0x4000,
            reserved3: 0,
            reserved4: 0,
            nand_rom_end: 0,
            nand_rw_start: 0,
            reserved5: [0; 24],
            reserved6: [0; 16],
            nin_logo: NIN_LOGO_DEFAULT,
            nin_logo_checksum: NIN_LOGO_CRC_DEFAULT,
            header_checksum: 0,
            dbg_rom_offset: 0,
            dbg_size: 0,
            dbg_ram_addr: 0
        }
    }
}

struct CPUHeaderData {
    rom_offset: u32,
    entry_addr: u32,
    ram_addr: u32,
    size: u32
}

struct Args {
    output_file_name: String,
    arm9_file_name: String,
    arm7_file_name: String,
}

fn wrong_input(msg: &str) -> ! {
    println!("{}", msg);
    std::process::exit(1);
}

fn print_help(opts: &Options) {
    print!("{}", opts.usage(&format!("IronDS ROM Tool version {}", VERSION.unwrap_or("unknown"))));
}

fn load_elf(elf_file_name: &str, output_file: &mut File) -> CPUHeaderData {
    // going to assume the start of the data should be 32-bit aligned
    file_align_32(output_file);
    let hdr_offset = output_file.stream_position().unwrap();

    let in_file = std::fs::read(Path::new(elf_file_name)).expect("Unable to open ARM9 executable");
    let elf_data = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(in_file.as_slice()).expect("no parsey");
    let elf_segments = elf_data.segments().unwrap();

    assert_eq!(elf_data.ehdr.e_machine, elf::abi::EM_ARM);

    let first_segment_addr =  elf_segments.get(0).unwrap().p_paddr;
    let mut last_segment_end = first_segment_addr;

    for s in elf_segments {
        // Skip non-loaded segments (that contain debug data or whatever)
        if s.p_type != elf::abi::PT_LOAD { continue; }

        // Skip empty segments
        if s.p_filesz == 0 || s.p_memsz == 0 { continue; }

        // if there's a gap in the paddrs, then there must be some bss section
        // in the middle, or something like that. it's probably a mistake,
        // but let's just pad some space for it.
        if s.p_paddr != last_segment_end {
            if s.p_paddr < last_segment_end {
                println!("ERROR: segment ending at p addr {:#010x} \
                    overlaps with segment starting at {:#010x}",last_segment_end, s.p_paddr);
                std::process::exit(1);
            }
            output_file.seek(SeekFrom::Current((s.p_paddr - last_segment_end) as i64)).unwrap();
        }
        last_segment_end = s.p_paddr + s.p_filesz;
        output_file.write(elf_data.segment_data(&s).unwrap()).unwrap();
    }
    CPUHeaderData { 
        rom_offset: hdr_offset as u32,
        entry_addr: elf_data.ehdr.e_entry as u32,
        ram_addr: first_segment_addr as u32,
        size: (output_file.stream_position().unwrap() - hdr_offset) as u32
    }
}

// Seek a file forward so that it is 32-bit aligned, filling in with 0s where necessary.
fn file_align_32(file: &mut File) {
    let cur_pos = file.stream_position().unwrap();

    // check if it's already aligned
    if cur_pos & 0b11 == 0 { return; }

    file.seek(SeekFrom::Current((0b11 - (cur_pos as i64 & 0b11)) + 1)).unwrap();
}

// Calculate the CRC-16 of a block of data, using the same algorithm as the DS BIOS.
// Uses the "MODBUS" type of CRC.
// https://problemkaputt.de/gbatek.htm#biosmiscfunctions
fn calc_crc_16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for d in data {
        crc = (crc >> 8) ^ CRC_16_TABLE[(crc as u8 ^ d) as usize];
    }
    crc
}

fn main() {
    let arglist: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.reqopt("o", "output", "Output NDS file", "<file.nds>");
    opts.reqopt("9", "arm9-exe", "ARM9 executable file", "<file.elf>");
    opts.reqopt("7", "arm7-exe", "ARM7 executable file", "<file.elf>");
    opts.optflag("h", "help", "Show this help");

    if arglist.len() < 2 || arglist.iter().any(|x| x == "-h" || x == "--help") {
        print_help(&opts);
        return;
    }

    let matches = match opts.parse(&arglist[1..]) {
        Ok(m) => { m }
        Err(f) => { wrong_input(&f.to_string()); }
    };
    let args = Args {
        output_file_name: matches.opt_str("o").unwrap(),
        arm9_file_name: matches.opt_str("9").unwrap(),
        arm7_file_name: matches.opt_str("7").unwrap(),
    };

    let output_file_path = Path::new(&args.output_file_name);
    let mut output_file = match File::options().create(true).read(true).write(true).open(output_file_path) {
        Err(why) => wrong_input(&format!("Unable to create file: {} - {}", output_file_path.display(), why)),
        Ok(file) => file
    };

    let mut header = Header::default();
    header.header_checksum = 0;

    // ARM9 binary starts at 0x4000
    output_file.seek(SeekFrom::Start(0x4000)).unwrap();
    let arm9_header = load_elf(&args.arm9_file_name, &mut output_file);
    // Gbatek says ARM7 binary has to start at a minimum offset of 0x8000
    if output_file.stream_position().unwrap() < 0x8000 {
        output_file.seek(SeekFrom::Start(0x8000)).unwrap();
    }
    let arm7_header = load_elf(&args.arm7_file_name, &mut output_file);

    header.arm9_rom_offset = arm9_header.rom_offset;
    header.arm9_entry_addr = arm9_header.entry_addr;
    header.arm9_ram_addr = arm9_header.ram_addr;
    header.arm9_size = arm9_header.size;
    header.arm7_rom_offset = arm7_header.rom_offset;
    header.arm7_entry_addr = arm7_header.entry_addr;
    header.arm7_ram_addr = arm7_header.ram_addr;
    header.arm7_size = arm7_header.size;

    // Get secure area checksum
    output_file.seek(SeekFrom::Start(header.arm9_rom_offset.into())).unwrap();
    let mut secure_buf = vec![0; (0x8000 - header.arm9_rom_offset) as usize];
    output_file.read_exact(&mut secure_buf).unwrap();
    header.secure_area_checksum = calc_crc_16(&secure_buf);

    // Get header checksum
    header.header_checksum = calc_crc_16(unsafe {
        std::slice::from_raw_parts(&header as *const Header as *const u8, 0x15E)
    });

    // Get total ROM size
    output_file.seek(SeekFrom::End(0)).unwrap();
    header.total_rom_size = output_file.stream_position().unwrap() as u32;

    output_file.seek(SeekFrom::Start(0)).unwrap();
    output_file.write(unsafe {
        std::slice::from_raw_parts(&header as *const Header as *const u8, std::mem::size_of::<Header>())
    }).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn file_align_32_already_aligned() {
        let mut tmpfile = tempfile::tempfile().unwrap();
        tmpfile.seek(SeekFrom::Start(4)).unwrap();
        file_align_32(&mut tmpfile);
        assert_eq!(tmpfile.stream_position().unwrap(), 4);
    }

    #[test]
    fn file_align_32_1_to_3() {
        let mut tmpfile = tempfile::tempfile().unwrap();
        for i in 1..4 {
            tmpfile.seek(SeekFrom::Start(i)).unwrap();
            file_align_32(&mut tmpfile);
            assert_eq!(tmpfile.stream_position().unwrap(), 4);
        }
    }

    #[test]
    fn crc_16_nin_logo() {
        let nin_logo = NIN_LOGO_DEFAULT;
        let crc = calc_crc_16(&nin_logo);
        assert_eq!(crc, NIN_LOGO_CRC_DEFAULT,
            "CRC-16 calculated as {:#06X}, should be {:#06X}", crc, NIN_LOGO_CRC_DEFAULT);
    }
}
