use std::fs;
use std::io::{self, Read};

pub fn run(args: &[String]) -> i32 {
    let mut check = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" | "--check" => check = true,
            _ => files.push(arg.as_str()),
        }
    }

    if check {
        return check_sums(&files);
    }

    if files.is_empty() {
        files.push("-");
    }

    let mut exit_code = 0;
    for file in files {
        match read_content(file) {
            Ok(data) => {
                let hash = md5_hash(&data);
                if file == "-" {
                    println!("{}  -", hash);
                } else {
                    println!("{}  {}", hash, file);
                }
            }
            Err(e) => {
                eprintln!("md5sum: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }
    exit_code
}

fn read_content(file: &str) -> io::Result<Vec<u8>> {
    if file == "-" {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        fs::read(file)
    }
}

fn check_sums(files: &[&str]) -> i32 {
    let mut exit_code = 0;
    for file in files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("md5sum: {}: {}", file, e);
                return 1;
            }
        };

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            if parts.len() != 2 {
                continue;
            }
            let expected = parts[0];
            let filename = parts[1];
            match fs::read(filename) {
                Ok(data) => {
                    let actual = md5_hash(&data);
                    if actual == expected {
                        println!("{}: OK", filename);
                    } else {
                        println!("{}: FAILED", filename);
                        exit_code = 1;
                    }
                }
                Err(e) => {
                    eprintln!("md5sum: {}: {}", filename, e);
                    exit_code = 1;
                }
            }
        }
    }
    exit_code
}

// Simple MD5 implementation
fn md5_hash(data: &[u8]) -> String {
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];

    let orig_len_bits = (data.len() as u64) * 8;

    // Padding
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_le_bytes());

    let s: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20,
        5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    let k: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };

            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(s[i]));
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    let bytes: Vec<u8> = state.iter().flat_map(|s| s.to_le_bytes()).collect();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
