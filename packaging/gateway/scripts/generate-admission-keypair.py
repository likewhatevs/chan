#!/usr/bin/env python3
"""Generate the raw Ed25519 admission keypair expected by gateway services."""

import base64
from pathlib import Path
import subprocess
import tempfile


ED25519_OID = b"\x2b\x65\x70"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_tlv(data: bytes, offset: int, expected_tag: int) -> tuple[bytes, int]:
    require(offset < len(data) and data[offset] == expected_tag, "unexpected DER tag")
    offset += 1
    require(offset < len(data), "missing DER length")
    length = data[offset]
    offset += 1
    if length & 0x80:
        count = length & 0x7F
        require(0 < count <= 4 and offset + count <= len(data), "invalid DER length")
        require(data[offset] != 0, "non-canonical DER length")
        length = int.from_bytes(data[offset : offset + count], "big")
        require(length >= 128, "non-canonical DER short length")
        offset += count
    end = offset + length
    require(end <= len(data), "truncated DER value")
    return data[offset:end], end


def only_tlv(data: bytes, expected_tag: int) -> bytes:
    value, end = read_tlv(data, 0, expected_tag)
    require(end == len(data), "trailing DER data")
    return value


def parse_algorithm(data: bytes) -> None:
    body = only_tlv(data, 0x30)
    oid, end = read_tlv(body, 0, 0x06)
    require(end == len(body) and oid == ED25519_OID, "key is not Ed25519")


def parse_private_key(data: bytes) -> bytes:
    body = only_tlv(data, 0x30)
    version, offset = read_tlv(body, 0, 0x02)
    require(version == b"\x00", "unexpected PKCS#8 version")
    algorithm, offset = read_tlv(body, offset, 0x30)
    parse_algorithm(bytes([0x30, len(algorithm)]) + algorithm)
    wrapped, offset = read_tlv(body, offset, 0x04)
    require(offset == len(body), "unexpected PKCS#8 fields")
    secret = only_tlv(wrapped, 0x04)
    require(len(secret) == 32, "Ed25519 secret is not 32 bytes")
    return secret


def parse_public_key(data: bytes) -> bytes:
    body = only_tlv(data, 0x30)
    algorithm, offset = read_tlv(body, 0, 0x30)
    parse_algorithm(bytes([0x30, len(algorithm)]) + algorithm)
    bits, offset = read_tlv(body, offset, 0x03)
    require(offset == len(body) and bits[:1] == b"\x00", "invalid public bit string")
    public = bits[1:]
    require(len(public) == 32, "Ed25519 public key is not 32 bytes")
    return public


def der_length(length: int) -> bytes:
    if length < 128:
        return bytes([length])
    encoded = length.to_bytes((length.bit_length() + 7) // 8, "big")
    return bytes([0x80 | len(encoded)]) + encoded


def tlv(tag: int, value: bytes) -> bytes:
    return bytes([tag]) + der_length(len(value)) + value


def pkcs8(secret: bytes) -> bytes:
    algorithm = tlv(0x30, tlv(0x06, ED25519_OID))
    return tlv(0x30, tlv(0x02, b"\x00") + algorithm + tlv(0x04, tlv(0x04, secret)))


def openssl(*args: str, input_data: bytes | None = None) -> bytes:
    result = subprocess.run(
        ["openssl", *args],
        input=input_data,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    )
    return result.stdout


def encode(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).rstrip(b"=").decode("ascii")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="chan-admission-key-") as directory:
        key_file = Path(directory) / "key.pem"
        subprocess.run(
            ["openssl", "genpkey", "-algorithm", "Ed25519", "-out", key_file],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=True,
        )
        private_der = openssl("pkey", "-in", str(key_file), "-outform", "DER")
        public_der = openssl(
            "pkey", "-in", str(key_file), "-pubout", "-outform", "DER"
        )

    secret = parse_private_key(private_der)
    public = parse_public_key(public_der)

    # Reconstructing PKCS#8 from the extracted bytes and deriving its public key
    # proves that the serialized secret and public values are the same keypair.
    rebuilt_public = parse_public_key(
        openssl("pkey", "-inform", "DER", "-pubout", "-outform", "DER", input_data=pkcs8(secret))
    )
    require(rebuilt_public == public, "extracted Ed25519 keys do not match")

    print(encode(secret))
    print(encode(public))


if __name__ == "__main__":
    main()
