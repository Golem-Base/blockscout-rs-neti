use std::ffi::OsString;

use alloy_rlp::Decodable;
use color_eyre::{Result, eyre::eyre};
use golem_base_sdk::entity::EncodableGolemBaseTransaction;

const HELP: &str = "\
Golem Base L3 storagetx parser

USAGE:
  storage-tx-parser hex-encoded-storage-tx-input

FLAGS:
  -h, --help            Prints help information
";

fn main() -> Result<()> {
    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains(["-h", "--help"]) {
        print!("{HELP}");
        std::process::exit(0);
    }

    let mut ops: Vec<String> = pargs
        .finish()
        .iter()
        .map(|o| o.clone().into_string())
        .collect::<Result<Vec<String>, OsString>>()
        .map_err(|_| eyre!("Non-utf8 arguments"))?;

    if ops.len() != 1 {
        print!("{HELP}");
        std::process::exit(0);
    }

    let bytes: Vec<u8> = hex::decode(ops.pop().unwrap())?;
    let tx = EncodableGolemBaseTransaction::decode(&mut bytes.as_slice())?;

    for create in tx.creates {
        let data: String = String::from_utf8(create.data.as_ref().to_vec())?;
        print!("create:\"{data}\":{}", create.btl);
        for ann in create.string_annotations {
            print!(":{}={}", ann.key, ann.value);
        }
        for ann in create.numeric_annotations {
            print!(":{}={}", ann.key, ann.value);
        }
        println!("");
    }
    for update in tx.updates {
        let data: String = String::from_utf8(update.data.as_ref().to_vec())?;
        print!(
            "update:0x{}:\"{data}\":{}",
            hex::encode(update.entity_key),
            update.btl
        );
        for ann in update.string_annotations {
            print!(":{}={}", ann.key, ann.value);
        }
        for ann in update.numeric_annotations {
            print!(":{}={}", ann.key, ann.value);
        }
        println!("");
    }
    for delete in tx.deletes {
        println!("delete:0x{}", hex::encode(delete));
    }
    for extend in tx.extensions {
        println!(
            "extend:0x{}:{}",
            hex::encode(extend.entity_key),
            extend.number_of_blocks
        );
    }
    Ok(())
}
