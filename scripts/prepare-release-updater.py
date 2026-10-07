"""Merge distribution-specific signed Linux packages into the Windows manifest."""
import json
from pathlib import Path
import sys
from urllib.parse import quote


def prepare(tag: str, manifest_path: Path, assets: Path) -> None:
    manifest = json.loads(manifest_path.read_text())
    if manifest["version"].lstrip("v") != tag.lstrip("v"):
        raise ValueError("Updater version does not match the release tag")
    targets = {
        "fedora": ("rpm", "linux-x86_64-rpm"),
        "ubuntu": ("deb", "linux-x86_64-ubuntu-deb"),
        "debian": ("deb", "linux-x86_64-debian-deb"),
    }
    for distro, (extension, target) in targets.items():
        packages = list(assets.glob(f"{tag}-{distro}-*.{extension}"))
        if len(packages) != 1:
            raise ValueError(f"Expected exactly one {distro} update package")
        package = packages[0]
        signature = Path(str(package) + ".sig").read_text().strip()
        if not signature:
            raise ValueError(f"Missing signature for {package.name}")
        manifest["platforms"][target] = {
            "url": f"https://github.com/V2P-Dev/Vispeak/releases/download/{tag}/{quote(package.name)}",
            "signature": signature,
        }
    manifest["notes"] = Path(f"docs/release_notes_{tag}.md").read_text()
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    prepare(sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3]))
