use std::ffi::OsString;

use alloy_rlp::Bytes;
use arkiv_storage_tx::{
    ChangeOwner, Create, Delete, Extend, NumericAttribute, StorageTransaction, StringAttribute,
    Update,
};
use color_eyre::{
    Result,
    eyre::{OptionExt, bail, eyre},
};

const HELP: &str = "\
Arkiv L3 test storagetx generator

USAGE:
  gen-test-data [operation1] [operation2] ...

FLAGS:
  -h, --help            Prints help information

ARGS:
  Possible operation formats:
    create:<data>:<btl>:<key=value>:<key2=value2>:...
    update:<key>:<data>:<btl>:<key=value>:<key2=value2>:...
    delete:<key>:<key2>:<key3>:...
    extend:<key>:<btl>
    change-owner:<key>:<new-owner>
";

#[derive(Clone)]
enum Operation {
    Create(Create),
    Delete(Delete),
    Update(Update),
    Extend(Extend),
    ChangeOwner(ChangeOwner),
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
        .fold(StorageTransaction::default(), |mut tx, op| {
            match op.clone() {
                Operation::Create(op) => tx.creates.push(op),
                Operation::Delete(op) => tx.deletes.push(op),
                Operation::Update(op) => tx.updates.push(op),
                Operation::Extend(op) => tx.extensions.push(op),
                Operation::ChangeOwner(op) => tx.change_owners.push(op),
            }
            tx
        });

    let buf: Bytes = tx.try_into()?;
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
        "change-owner" => Operation::ChangeOwner(parse_change_owner_op(parts.collect())?),
        _ => bail!("Unknown operation type"),
    })
}

fn parse_string_attributes(parts: &[&str]) -> Vec<StringAttribute> {
    parts
        .iter()
        .filter_map(|v| {
            let (key, value) = v.split_once('=')?;
            if value.parse::<u64>().is_ok() {
                return None;
            }
            Some(StringAttribute {
                key: key.into(),
                value: value.into(),
            })
        })
        .collect()
}

fn parse_numeric_attributes(parts: &[&str]) -> Vec<NumericAttribute> {
    parts
        .iter()
        .filter_map(|v| {
            let (key, value) = v.split_once('=')?;
            value
                .parse::<u64>()
                .map(|value| NumericAttribute {
                    key: key.into(),
                    value,
                })
                .ok()
        })
        .collect()
}

fn parse_create_op(parts: Vec<&str>) -> Result<Create> {
    let string_attributes = parse_string_attributes(&parts[2..]);
    let numeric_attributes = parse_numeric_attributes(&parts[2..]);
    Ok(Create {
        payload: parts[0].to_string().into(),
        content_type: "plain/text".into(),
        btl: parts[1].parse()?,
        string_attributes,
        numeric_attributes,
    })
}

fn parse_update_op(parts: Vec<&str>) -> Result<Update> {
    let string_attributes = parse_string_attributes(&parts[3..]);
    let numeric_attributes = parse_numeric_attributes(&parts[3..]);
    Ok(Update {
        entity_key: parts[0].parse()?,
        payload: parts[1].to_string().into(),
        content_type: "plain/text".into(),
        btl: parts[2].parse()?,
        string_attributes,
        numeric_attributes,
    })
}

fn parse_delete_op(parts: Vec<&str>) -> Result<Delete> {
    Ok(parts[0].parse()?)
}

fn parse_extend_op(parts: Vec<&str>) -> Result<Extend> {
    Ok(Extend {
        entity_key: parts[0].parse()?,
        number_of_blocks: parts[1].parse()?,
    })
}

fn parse_change_owner_op(parts: Vec<&str>) -> Result<ChangeOwner> {
    Ok(ChangeOwner {
        entity_key: parts[0].parse()?,
        new_owner: parts[1].parse()?,
    })
}
