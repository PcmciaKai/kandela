"""Writes flatpak-builder sources that vendor every crates.io dependency in a
Cargo.lock, so `cargo --offline` works inside the Flatpak build sandbox.

Usage: python3 gen_cargo_sources.py Cargo.lock out.json
"""

import json
import sys
import tomllib

CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"


def main(lock_path: str, out_path: str) -> None:
    with open(lock_path, "rb") as f:
        lock = tomllib.load(f)

    sources = []
    for pkg in lock["package"]:
        source = pkg.get("source")
        if source is None:
            continue  # a workspace member
        if source != CRATES_IO:
            sys.exit(f"unsupported source for {pkg['name']}: {source}")
        name, version, checksum = pkg["name"], pkg["version"], pkg["checksum"]
        dest = f"cargo/vendor/{name}-{version}"
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
            "sha256": checksum,
            "dest": dest,
        })
        # Cargo checks vendored crates against this; the archive hash is enough.
        sources.append({
            "type": "inline",
            "contents": json.dumps({"package": checksum, "files": {}}),
            "dest": dest,
            "dest-filename": ".cargo-checksum.json",
        })

    sources.append({
        "type": "inline",
        "contents": (
            '[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n'
            '[source.crates-io]\nreplace-with = "vendored-sources"\n'
        ),
        "dest": "cargo",
        "dest-filename": "config.toml",
    })

    with open(out_path, "w") as f:
        json.dump(sources, f, indent=4)
        f.write("\n")
    print(f"{len(sources) // 2} crates")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
