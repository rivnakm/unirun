import os
import semver
import tomllib


def read_version() -> semver.Version:
    data = {}
    with open("Cargo.toml", "rb") as f:
        data = tomllib.load(f)

    version: str = data["package"]["version"]
    return semver.Version.parse(version)


def github_output(key: str, value: str):
    if "GITHUB_OUTPUT" not in os.environ:
        print(f"{key}={value}")
        return

    output = os.environ["GITHUB_OUTPUT"]

    with open(output, "a") as f:
        _ = f.write(f"{key}={value}")


def main():
    version = read_version()

    github_output("version", str(version))
    github_output("prerelease", str(bool(version.prerelease)).lower())


if __name__ == "__main__":
    main()
