use rand::Rng;

pub const fn fnv1a_hash(s: &str) -> u64 {
    let bytes = s.as_bytes();
    let mut hash = 0xcbf29ce484222325u64;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

#[macro_export]
macro_rules! ct_hash {
    ($s:expr) => {{
        const _HASH: u64 = $crate::obfuscation::fnv1a_hash($s);
        _HASH
    }};
}

pub struct ObfStr {
    data: Vec<u8>,
    key: u8,
}

impl ObfStr {
    pub fn new(s: &str) -> Self {
        let mut rng = rand::thread_rng();
        let key = rng.r#gen::<u8>();
        let data = s.bytes().map(|b| b ^ key).collect();
        Self { data, key }
    }

    pub fn reveal(&self) -> String {
        let bytes: Vec<u8> = self.data.iter().map(|b| b ^ self.key).collect();
        String::from_utf8_lossy(&bytes).to_string()
    }
}

#[macro_export]
macro_rules! obf_string {
    ($s:expr) => {{
        let obf = $crate::obfuscation::ObfStr::new($s);
        obf.reveal()
    }};
}

pub fn multilayer_encrypt(data: &[u8], layers: usize) -> Vec<u8> {
    let mut result = data.to_vec();
    for layer in 0..layers {
        let mut key = 0xA7u8.wrapping_add(layer as u8);
        for byte in result.iter_mut() {
            *byte ^= key;
            key = key.wrapping_mul(13).wrapping_add(47);
        }
    }
    result
}

pub fn multilayer_decrypt(data: &[u8], layers: usize) -> Vec<u8> {
    let mut result = data.to_vec();
    for layer in (0..layers).rev() {
        let mut key = 0xA7u8.wrapping_add(layer as u8);
        for byte in result.iter_mut() {
            *byte ^= key;
            key = key.wrapping_mul(13).wrapping_add(47);
        }
    }
    result
}

pub struct StackString<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> StackString<N> {
    pub fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
        }
    }

    pub fn from_encrypted(encrypted: &[u8], key: u8) -> Self {
        let mut s = Self::new();
        for (i, &byte) in encrypted.iter().enumerate().take(N) {
            s.buf[i] = byte ^ key;
            s.len += 1;
        }
        s
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

pub type FnPtr = fn() -> u64;

pub fn indirect_call(func: FnPtr) -> u64 {
    let ptr = func as usize;
    let mangled = ptr ^ 0xDEADBEEFCAFEBABE;
    let restored = mangled ^ 0xDEADBEEFCAFEBABE;
    let restored_fn: FnPtr = unsafe { std::mem::transmute(restored) };
    restored_fn()
}

pub fn scramble_int(val: i32) -> i32 {
    let mut result = val;
    result ^= 0x5A5A5A5A;
    result = result.wrapping_mul(0x45D9F3B);
    result = result.rotate_left(13);
    result ^= 0xAAAAAAAAu32 as i32;
    result
}

pub fn unscramble_int(val: i32) -> i32 {
    let inv = 0x119DE1F3i32;
    let mut result = val;
    result ^= 0xAAAAAAAAu32 as i32;
    result = result.rotate_right(13);
    result = result.wrapping_mul(inv);
    result ^= 0x5A5A5A5A;
    result
}

#[macro_export]
macro_rules! switch_case {
    ($val:expr, $( $case:expr => $code:expr ),* $(,)?) => {{
        let computed = $val;
        $(
            if computed == $case {
                $crate::obfuscation::stack_noise();
                return $code;
            }
        )*
        panic!("unhandled case");
    }};
}

pub fn junk_code_1() -> u32 {
    let mut x = 0u32;
    for i in 0..100 {
        x = x.wrapping_add(i);
        x = x.wrapping_mul(3);
        x ^= 0xDEADBEEF;
    }
    x
}

pub fn junk_code_2() -> u64 {
    let mut val = 0x1234567890ABCDEFu64;
    for _ in 0..50 {
        val = val.rotate_left(7);
        val ^= 0xFEDCBA9876543210;
        val = val.wrapping_mul(0x123456789);
    }
    val
}

#[macro_export]
macro_rules! confuse_flow {
    ($code:block) => {{
        let r = rand::random::<u8>() % 3;
        match r {
            0 => {
                $crate::obfuscation::junk_code_1();
                $code
            }
            1 => {
                $crate::obfuscation::junk_code_2();
                $code
            }
            _ => {
                $crate::obfuscation::junk_code_1();
                $crate::obfuscation::junk_code_2();
                $code
            }
        }
    }};
}

pub fn fake_operations(input: u64) -> u64 {
    let mut result = input;
    result ^= 0xAAAAAAAAAAAAAAAA;
    result = result.wrapping_mul(0x5555555555555555);
    result = result.rotate_left(13);
    result ^= 0x3333333333333333;
    result
}

#[inline(never)]
pub fn opaque_predicate() -> bool {
    let x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (x & 1) == (x & 1)
}

pub fn stack_noise() {
    let mut buf = [0u8; 256];
    for i in 0..256 {
        buf[i] = (i as u8).wrapping_mul(137);
        std::hint::black_box(buf[i]);
    }
    std::hint::black_box(&buf);
}

#[macro_export]
macro_rules! hide_call {
    ($func:expr) => {{
        if $crate::obfuscation::opaque_predicate() {
            $crate::obfuscation::stack_noise();
            $func
        } else {
            unreachable!()
        }
    }};
}

pub fn xor_buffer(data: &mut [u8], key: u8) {
    for byte in data.iter_mut() {
        *byte ^= key;
    }
}

pub fn encode_value(val: u64) -> u64 {
    let k1 = 0x87654321DEADBEEF;
    let k2 = 0x123456789ABCDEF0;
    val.wrapping_mul(k1).wrapping_add(k2)
}

pub fn decode_value(val: u64) -> u64 {
    let k1 = 0x87654321DEADBEEF;
    let k2 = 0x123456789ABCDEF0;
    val.wrapping_sub(k2).wrapping_mul(mod_inverse(k1))
}

fn mod_inverse(a: u64) -> u64 {
    let mut t = 0i128;
    let mut newt = 1i128;
    let mut r = u64::MAX as i128 + 1;
    let mut newr = a as i128;

    while newr != 0 {
        let quotient = r / newr;
        let temp = t - quotient * newt;
        t = newt;
        newt = temp;
        let temp = r - quotient * newr;
        r = newr;
        newr = temp;
    }

    if r > 1 {
        return a;
    }
    if t < 0 {
        t += u64::MAX as i128 + 1;
    }
    t as u64
}

