use getopts::Options;
use std::env;
use std::io::{Seek, Write};
use std::fs::File;
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

// # https://problemkaputt.de/gbatek.htm#dscartridgeheader
#[repr(C, packed)]
struct Header {
    game_title: [u8; 12],       // (Uppercase ASCII, padded with 00h)
    game_code: [u8; 4],         // (Uppercase ASCII, NTR-<code>)        (0=homebrew)
    maker_code: [u8; 2],        // (Uppercase ASCII, eg. "01"=Nintendo) (0=homebrew)
    unit_code: [u8; 2],         // (00h=NDS, 02h=NDS+DSi, 03h=DSi) (bit1=DSi)
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

fn load_elf(elf_file_name: &str, output_file: &mut File) {
    // going to assume the start of the ARM7 data should be 32-bit aligned
    file_align_32(output_file);

    let arm9_path = std::path::PathBuf::from(elf_file_name);
    let arm9_file = std::fs::read(arm9_path).expect("Unable to open ARM9 executable");
    let arm9_elf = elf::ElfBytes::<elf::endian::AnyEndian>::minimal_parse(arm9_file.as_slice()).expect("no parsey");
    let arm9_segments = arm9_elf.segments().unwrap();

    let mut last_segment_end: u64 = arm9_segments.get(0).unwrap().p_paddr;

    for s in arm9_segments {
        // Skip non-loaded segments (that contain debug data or whatever)
        if s.p_type != elf::abi::PT_LOAD { continue; }

        // Skip empty segments
        if s.p_filesz == 0 || s.p_memsz == 0 { continue; }

        println!("{:?}", s);

        if s.p_paddr != last_segment_end {
            println!("gap size: {}", s.p_paddr - last_segment_end);
        }
        println!("{:#02x}", s.p_memsz);
        last_segment_end = s.p_paddr + s.p_memsz;
        output_file.write(&[5]).unwrap();
    }
}

// Seek a file forward so that it is 32-bit aligned, filling in with 0s where necessary.
fn file_align_32(file: &mut File) {
    let cur_pos = file.stream_position().unwrap();

    // check if it's already aligned
    if cur_pos & 0b11 == 0 { return; }

    file.seek(std::io::SeekFrom::Current((0b11 - (cur_pos as i64 & 0b11)) + 1)).unwrap();
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

    let output_file_path = std::path::Path::new(&args.output_file_name);
    let mut output_file = match File::create(output_file_path) {
        Err(why) => wrong_input(&format!("Unable to create file: {} - {}", output_file_path.display(), why)),
        Ok(file) => file
    };

    // ARM9 binary starts at 0x4000
    output_file.seek(std::io::SeekFrom::Start(0x4000)).unwrap();

    println!("Arm9");
    load_elf(&args.arm9_file_name, &mut output_file);
    println!("Arm7");
    // Gbatek says ARM7 binary has to start at a minimum offset of 0x8000
    if output_file.stream_position().unwrap() < 0x8000 {
        output_file.seek(std::io::SeekFrom::Start(0x8000)).unwrap();
    }
    load_elf(&args.arm7_file_name, &mut output_file);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile;

    #[test]
    fn file_align_32_already_aligned() {
        let mut tmpfile = tempfile::tempfile().unwrap();
        tmpfile.seek(std::io::SeekFrom::Start(4)).unwrap();
        file_align_32(&mut tmpfile);
        assert_eq!(tmpfile.stream_position().unwrap(), 4);
    }

    #[test]
    fn file_align_32_1_to_3() {
        let mut tmpfile = tempfile::tempfile().unwrap();
        for i in 1..4 {
            tmpfile.seek(std::io::SeekFrom::Start(i)).unwrap();
            file_align_32(&mut tmpfile);
            assert_eq!(tmpfile.stream_position().unwrap(), 4);
        }
    }
}
