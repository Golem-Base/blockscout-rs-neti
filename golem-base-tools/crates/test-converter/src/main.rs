use alloy_primitives::Bytes;
use alloy_rlp::Decodable;
use arkiv_storage_tx::{
    Create, Extend, NumericAttribute, StorageTransaction, StringAttribute, Update,
};
use color_eyre::Result;
use golem_base_sdk::entity::EncodableGolemBaseTransaction;
use std::fs::File;
use std::io::{BufRead, Write};

const HELP: &str = "\
Internal

USAGE:
  test-converter fixture.sql

FLAGS:
  -h, --help            Prints help information
";

fn main() -> Result<()> {
    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains(["-h", "--help"]) {
        print!("{HELP}");
        std::process::exit(0);
    }

    let args = pargs.finish();
    let path = args.first().expect("Provide path");
    let tmpfile = tempfile::NamedTempFile::new()?;
    let tmppath = tmpfile.path().to_owned();
    let mut writer = std::io::BufWriter::new(tmpfile);
    {
        let file = File::open(path)?;
        for line in std::io::BufReader::new(file).lines() {
            let line = process_line(&line.unwrap())?;
            writeln!(writer, "{line}")?;
        }
    }
    std::fs::rename(tmppath, path)?;
    Ok(())
}

fn process_line(line: &str) -> Result<String> {
    let line = line
        .replace(
            "0000000000000000000000000000000060138453",
            "00000000000000000000000000000061726B6976",
        )
        // entity deleted sig
        .replace(
            "0297b0e6eaf1bc2289906a8123b8ff5b19e568a60d002d47df44f8294422af93",
            "749d62eff980a5016f4f357bd7eb8b65163f1e25bc400dcfc5e33f0e7910149e",
        )
        // entity extended sig
        .replace(
            "835bfca6df78ffac92635dcc105a6a8c4fd715e054e18ef60448b0a6dce30c8d",
            "0a5f98a4e3c7ac5f503e302ccd21b6132f04d51b89c5e02487c89ab3b7c6d60b",
        );
    if !line
        .trim_start()
        .starts_with("INSERT INTO public.transactions")
        && !line.trim_start().starts_with("INSERT INTO transactions")
    {
        return Ok(line.into());
    }

    let parts: Vec<String> = line.split(",").map(|v| v.to_string()).collect();
    let input_idx = 37;
    let input_len = parts[input_idx].trim().len();
    let input = &parts[input_idx].trim()[3..input_len - 1];
    let input_hex: Vec<u8> = hex::decode(input)?;
    let tx = match EncodableGolemBaseTransaction::decode(&mut input_hex.as_slice()) {
        Ok(v) => v,
        Err(e) => {
            println!("Couldn't parse {input} - {e}, skipping...");
            return Ok(line.into());
        }
    };
    let tx = StorageTransaction {
        creates: tx
            .creates
            .into_iter()
            .map(|v| Create {
                btl: v.btl,
                content_type: "text/plain".into(),
                payload: v.data,
                string_attributes: v
                    .string_annotations
                    .into_iter()
                    .map(|v| StringAttribute {
                        key: v.key,
                        value: v.value,
                    })
                    .collect(),
                numeric_attributes: v
                    .numeric_annotations
                    .into_iter()
                    .map(|v| NumericAttribute {
                        key: v.key,
                        value: v.value,
                    })
                    .collect(),
            })
            .collect(),
        updates: tx
            .updates
            .into_iter()
            .map(|v| Update {
                entity_key: v.entity_key,
                btl: v.btl,
                content_type: "text/plain".into(),
                payload: v.data,
                string_attributes: v
                    .string_annotations
                    .into_iter()
                    .map(|v| StringAttribute {
                        key: v.key,
                        value: v.value,
                    })
                    .collect(),
                numeric_attributes: v
                    .numeric_annotations
                    .into_iter()
                    .map(|v| NumericAttribute {
                        key: v.key,
                        value: v.value,
                    })
                    .collect(),
            })
            .collect(),
        deletes: tx.deletes,
        extensions: tx
            .extensions
            .into_iter()
            .map(|v| Extend {
                entity_key: v.entity_key,
                number_of_blocks: v.number_of_blocks,
            })
            .collect(),
        change_owners: vec![],
    };
    let fixed_input: Bytes = tx.try_into()?;
    let fixed_input = format!("'\\x{}'", hex::encode(fixed_input));
    let mut parts = parts;
    parts[input_idx] = fixed_input;
    Ok(parts.join(","))
}
