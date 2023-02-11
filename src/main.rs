use std::path::Path;
use getopts::Options;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

struct Args {
    output_file_name: String,
    arm9_file_name: String,
    arm7_file_name: String,
}

fn print_help(opts: &Options) {
    print!("{}", opts.usage(&format!("IronDS ROM Tool version {}", VERSION.unwrap_or("unknown"))));
}

fn main() -> Result<(), String> {
    let arglist: Vec<String> = std::env::args().collect();
    let mut opts = Options::new();
    opts.reqopt("o", "output", "Output NDS file", "<file.nds>");
    opts.reqopt("9", "arm9-exe", "ARM9 executable file", "<file.elf>");
    opts.reqopt("7", "arm7-exe", "ARM7 executable file", "<file.elf>");
    opts.optflag("h", "help", "Show this help");

    if arglist.len() < 2 || arglist.iter().any(|x| x == "-h" || x == "--help") {
        print_help(&opts);
        return Ok(());
    }

    let matches = match opts.parse(&arglist[1..]) {
        Ok(m) => { m }
        Err(f) => { return Err(f.to_string()); }
    };
    let args = Args {
        output_file_name: matches.opt_str("o").unwrap(),
        arm9_file_name: matches.opt_str("9").unwrap(),
        arm7_file_name: matches.opt_str("7").unwrap(),
    };

    let output_path = Path::new(&args.output_file_name);
    let arm9_path = Path::new(&args.arm9_file_name);
    let arm7_path = Path::new(&args.arm7_file_name);

    ironds_romtool::build_rom(output_path, arm9_path, arm7_path)
}
