import hashlib
import binascii
import itertools

target_hash = "af5dbd2edbc1af02fc722df4cfc02c969aef276fb23d104e19b78a35257520b9"
salt_hex = "2ed2e3b4822084aef82c8d68d7b04b79"
salt = binascii.unhexlify(salt_hex)

# Common PINs
common_pins = [
    "000000", "111111", "123456", "654321", "123123", "999999", "888888", "123321", "0000", "1234"
]

print("Checking common PINs...")
for pin in common_pins:
    key = hashlib.pbkdf2_hmac('sha256', pin.encode(), salt, 310000, 32)
    pin_hash = hashlib.sha256(key).hexdigest()
    if pin_hash == target_hash:
        print(f"FOUND PIN: {pin}")
        exit(0)

print("Common PINs failed. Not brute-forcing full space yet.")
