use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let input = PathBuf::from(args.next().context("missing .aix file")?);
    let raw = fs::read_to_string(&input)?;
    let doc = parse(&raw)?;
    validate(&doc)?;
    let canonical = canonicalize(&doc);

    match command.as_str() {
        "inspect" => {
            println!("module: {}", doc["module"]);
            println!("input: {} : {}", doc["input.0.name"], doc["input.0.type"]);
            println!("domain: {}", domain_summary(&doc)?);
            println!("sha256: {}", sha256_hex(canonical.as_bytes()));
        }
        "canonical" => {
            if args.next().as_deref() != Some("--out") {
                bail!("usage: axiom-ir canonical <input.aix> --out <output.aix>");
            }
            let out = PathBuf::from(args.next().context("missing output path")?);
            fs::write(out, canonical)?;
        }
        _ => bail!("usage: axiom-ir <inspect|canonical> <input.aix> [--out path]"),
    }

    Ok(())
}

fn parse(raw: &str) -> Result<BTreeMap<String, String>> {
    let mut lines = raw.lines();
    if lines.next() != Some("AXIOM-IR/2") {
        bail!("invalid or unsupported IR header; expected AXIOM-IR/2");
    }

    let mut doc = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("line {} has no '='", index + 2))?;
        if doc
            .insert(key.to_owned(), value.trim().to_owned())
            .is_some()
        {
            bail!("duplicate key: {key}");
        }
    }
    Ok(doc)
}

fn validate(doc: &BTreeMap<String, String>) -> Result<()> {
    for key in [
        "module",
        "input.0.name",
        "input.0.type",
        "output.name",
        "output.type",
    ] {
        if !doc.contains_key(key) {
            bail!("missing required key: {key}");
        }
    }

    if doc["input.0.type"] != "int" || doc["output.type"] != "int" {
        bail!("IR/2 symbolic core supports int -> int only");
    }
    if !doc
        .keys()
        .any(|key| key.starts_with("ensures.") && key.ends_with(".expr"))
    {
        bail!("specification has no ensures expressions");
    }

    let input = &doc["input.0.name"];
    let kind_key = format!("domain.{input}.kind");
    match doc.get(&kind_key).map(String::as_str) {
        Some("unbounded") => {}
        Some("range") => {
            let min: i64 = doc
                .get(&format!("domain.{input}.min"))
                .context("range domain misses min")?
                .parse()?;
            let max: i64 = doc
                .get(&format!("domain.{input}.max"))
                .context("range domain misses max")?
                .parse()?;
            if min > max {
                bail!("invalid range domain: {min}..{max}");
            }
        }
        Some(other) => bail!("unsupported domain kind: {other}"),
        None => bail!("missing domain kind for input {input}"),
    }

    Ok(())
}

fn canonicalize(doc: &BTreeMap<String, String>) -> String {
    let mut out = String::from("AXIOM-IR/2\n");
    for (key, value) in doc {
        out.push_str(key);
        out.push('=');
        out.push_str(value.trim());
        out.push('\n');
    }
    out
}

fn domain_summary(doc: &BTreeMap<String, String>) -> Result<String> {
    let input = &doc["input.0.name"];
    let kind = doc
        .get(&format!("domain.{input}.kind"))
        .context("missing domain kind")?;
    if kind == "unbounded" {
        return Ok("unbounded integers".to_owned());
    }
    Ok(format!(
        "{}..{}",
        doc[&format!("domain.{input}.min")],
        doc[&format!("domain.{input}.max")]
    ))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_unbounded_document() {
        let doc = parse(
            "AXIOM-IR/2\nmodule=a\ninput.0.name=x\ninput.0.type=int\noutput.name=result\noutput.type=int\ndomain.x.kind=unbounded\nensures.0.expr=result == x\n",
        )
        .unwrap();
        assert!(validate(&doc).is_ok());
    }
}
