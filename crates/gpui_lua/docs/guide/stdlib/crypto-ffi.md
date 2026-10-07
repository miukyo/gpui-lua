# Cryptography & FFI (`crypto`, `ffi`)

Cryptographic hashing, UUID generation, Base64 encoding, and dynamic library FFI.

---

## Cryptography (`crypto`)

```lua
-- SHA-256 and SHA-512 hashes (hex strings)
local hash = crypto.sha256("my_secret_data")
local hash512 = crypto.sha512("my_secret_data")

-- HMAC-SHA256
local signature = crypto.hmac_sha256("secret_key", "payload")

-- Random UUID v4
local id = crypto.uuid()

-- Base64 encoding and decoding
local encoded = crypto.base64_encode("Hello World")
local decoded = crypto.base64_decode(encoded)

-- Cryptographically secure random bytes (hex)
local salt = crypto.random_bytes(16)
```

---

## Foreign Function Interface (`ffi`)

Load dynamic C libraries (`.dll`, `.so`, `.dylib`) directly from Lua:

```lua
local lib = ffi.load("my_c_library.dll")

-- Call exported C function symbol
local result = lib:call("add", 10, 20)
print("Result from C:", result)

-- Check host architecture
print(ffi.os)   -- "windows", "macos", "linux"
print(ffi.arch) -- "x86_64", "aarch64"

-- Release library handle
lib:close()
```
