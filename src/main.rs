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
            println!("module: {}", doc.get("module").unwrap());
            println!(
                "domain: {}..{}",
                doc.get("domain.min").unwrap(),
                doc.get("domain.max").unwrap()
            );
            println!("sha256: {}", hex_sha256(canonical.as_bytes()));
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
    if lines.next() != Some("AXIOM-IR/1") {
        bail!("invalid or unsupported IR header");
    }
    let mut doc = BTreeMap::new();
    for (i, line) in lines.enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .with_context(|| format!("line {} has no '='", i + 2))?;
        if doc.insert(key.to_owned(), value.to_owned()).is_some() {
            bail!("duplicate key: {key}");
        }
    }
    Ok(doc)
}

fn validate(doc: &BTreeMap<String, String>) -> Result<()> {
    for key in [
        "module",
        "input.name",
        "input.type",
        "output.name",
        "output.type",
        "domain.min",
        "domain.max",
    ] {
        if !doc.contains_key(key) {
            bail!("missing required key: {key}");
        }
    }
    if doc.get("input.type").map(String::as_str) != Some("i64")
        || doc.get("output.type").map(String::as_str) != Some("i64")
    {
        bail!("IR/1 supports i64 -> i64 only");
    }
    let min: i64 = doc["domain.min"].parse()?;
    let max: i64 = doc["domain.max"].parse()?;
    if min > max {
        bail!("invalid domain: {min}..{max}");
    }
    if !doc.keys().any(|k| k.starts_with("ensures.")) {
        bail!("specification has no ensures clauses");
    }
    Ok(())
}

fn canonicalize(doc: &BTreeMap<String, String>) -> String {
    let mut s = String::from("AXIOM-IR/1\n");
    for (k, v) in doc {
        s.push_str(k);
        s.push('=');
        s.push_str(v.trim());
        s.push('\n');
    }
    s
}

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_keys() {
        assert!(parse("AXIOM-IR/1\nmodule=a\nmodule=b\n").is_err());
    }
}
