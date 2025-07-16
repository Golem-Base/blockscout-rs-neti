use std::ffi::OsString;

use alloy_rlp::{Bytes, encode};
use color_eyre::{
    Result,
    eyre::{OptionExt, bail, eyre},
};
use golem_base_sdk::{
    NumericAnnotation, StringAnnotation,
    entity::{Create, EncodableGolemBaseTransaction, Extend, GolemBaseDelete, Update},
};

const HELP: &str = "\
Golem Base L3 test storagetx generator

USAGE:
  gen-test-data [operation1] [operation2] ...

FLAGS:
  -h, --help            Prints help information

ARGS:
  Possible operation formats:
    create:<data>:<btl>:<key=value>:<key2=value2>
    update:<key>:<data>:<btl>:<key=value>:<key2=value2>
    delete:<key>
    extend:<key>:<btl>
";

#[derive(Clone)]
enum Operation {
    Create(Create),
    Delete(GolemBaseDelete),
    Update(Update),
    Extend(Extend),
}

fn main() -> Result<()> {
    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains(["-h", "--help"]) {
        print!("{HELP}");
        std::process::exit(0);
    }

    let ops: Vec<String> = pargs
        .finish()
        .iter()
        .map(|o| o.clone().into_string())
        .collect::<Result<Vec<String>, OsString>>()
        .map_err(|_| eyre!("Non-utf8 arguments"))?;

    if ops.is_empty() {
        print!("{HELP}");
        std::process::exit(0);
    }

    let tx = ops
        .iter()
        .map(|op| parse_op(op))
        .collect::<Result<Vec<Operation>>>()?
        .iter()
        .fold(EncodableGolemBaseTransaction::default(), |mut tx, op| {
            match op.clone() {
                Operation::Create(op) => tx.creates.push(op),
                Operation::Delete(op) => tx.deletes.push(op),
                Operation::Update(op) => tx.updates.push(op),
                Operation::Extend(op) => tx.extensions.push(op),
            }
            tx
        });

    let buf: Bytes = encode(tx).into();
    println!("0x{buf:x}");
    Ok(())
}

fn parse_op(s: &str) -> Result<Operation> {
    let mut parts = s.split(':');
    let optype = parts.next().ok_or_eyre("Invalid operation spec")?;
    Ok(match optype {
        "create" => Operation::Create(parse_create_op(parts.collect())?),
        "update" => Operation::Update(parse_update_op(parts.collect())?),
        "delete" => Operation::Delete(parse_delete_op(parts.collect())?),
        "extend" => Operation::Extend(parse_extend_op(parts.collect())?),
        _ => bail!("Unknown operation type"),
    })
}

fn parse_string_annotations(parts: &[&str]) -> Vec<StringAnnotation> {
    parts
        .iter()
        .filter_map(|v| {
            let (key, value) = v.split_once('=')?;
            if value.parse::<u64>().is_ok() {
                return None;
            }
            Some(StringAnnotation {
                key: key.into(),
                value: value.into(),
            })
        })
        .collect()
}

fn parse_numeric_annotations(parts: &[&str]) -> Vec<NumericAnnotation> {
    parts
        .iter()
        .filter_map(|v| {
            let (key, value) = v.split_once('=')?;
            value
                .parse::<u64>()
                .map(|value| NumericAnnotation {
                    key: key.into(),
                    value,
                })
                .ok()
        })
        .collect()
}

fn parse_create_op(parts: Vec<&str>) -> Result<Create> {
    let string_annotations = parse_string_annotations(&parts[2..]);
    let numeric_annotations = parse_numeric_annotations(&parts[2..]);
    Ok(Create {
        data: parts[0].to_string().into(),
        btl: parts[1].parse()?,
        string_annotations,
        numeric_annotations,
    })
}

fn parse_update_op(parts: Vec<&str>) -> Result<Update> {
    let string_annotations = parse_string_annotations(&parts[3..]);
    let numeric_annotations = parse_numeric_annotations(&parts[3..]);
    Ok(Update {
        entity_key: parts[0].parse()?,
        data: parts[1].to_string().into(),
        btl: parts[2].parse()?,
        string_annotations,
        numeric_annotations,
    })
}

fn parse_delete_op(parts: Vec<&str>) -> Result<GolemBaseDelete> {
    Ok(parts[0].parse()?)
}

fn parse_extend_op(parts: Vec<&str>) -> Result<Extend> {
    Ok(Extend {
        entity_key: parts[0].parse()?,
        number_of_blocks: parts[1].parse()?,
    })
}
