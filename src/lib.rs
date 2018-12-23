// based on: http://www.amnoid.de/gc/yaz0.txt <3 and libyaz0: https://github.com/aboood40091/libyaz0
#[cfg(test)] extern crate rand;
#[cfg(test)] extern crate pretty_assertions;

extern crate byteorder;
use byteorder::{ReadBytesExt, WriteBytesExt, BE};
use std::io::Cursor;
use std::io::SeekFrom;
use std::io::prelude::*;
use std::iter::FromIterator;

// say you have a vector and only need from vec[5:7] you can do this via this
// function and yes that thing is not really effective ....
pub fn get_subvector(base: &Vec<u8>, from: usize, till: usize) -> Vec<u8> {
    if from >= till {
        return vec![];
    } else if from > base.len() {
        return vec![];
    } else if till > base.len() {
        return vec![];
    }
    Vec::from_iter(base[from..till].iter().cloned())
}

// TODO: test_decompress ?!
// TODO: I think I need to add a Yaz0 magic?!
// TODO: compress is broken! It takes like forever!
#[cfg(test)]
mod tests {
    #[test]
    fn test_get_subvector() {
        let ret = super::get_subvector(&vec![1, 2, 3, 4, 5, 6, 7, 8, 9], 3, 7);
        assert_eq!(ret, [4, 5, 6, 7]);
    }
    #[test]
    fn test_compress() {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();

        // executes for all compression levels:
        for i in 0..9 {
            // generate a random block of data to test with:
            let mut block: Vec<u8> = Vec::new();
            while block.len() != 2048 {
                block.push(rng.gen_range(0, 255));
            }
            // run the test:
            super::compress(block, i);
        }
    }
}


// this function searches for recurring sequences?!
fn compression_search(buffer: &Vec<u8>, pos: usize, max_len: usize, search_range: usize, src_end: usize) -> (usize, usize) {
    let mut found_len = 1;
    let mut found = 0;
    // println!("search_range: {}", search_range);
    // println!("pos: {}", pos);
    // println!("max_len: {}", max_len);
    // println!("src_end: {}", src_end);
    if pos + 2 < src_end {
        let mut search: isize = pos as isize - search_range as isize;
        if search < 0 {
            search = 0;
        }

        let mut cmp_end = pos + max_len;
        if cmp_end > src_end {
            cmp_end = src_end;
        }
        let c1 = get_subvector(&buffer, pos, pos+1);
        while search < pos as isize {
            search = { // TODO: this might be broken!
                // it should implement pythons str.find(sub[, start[, end]] )
                let mut result: isize = -1;
                for i in search..pos as isize{
                    if c1 == get_subvector(&buffer, i as usize, c1.len()) {
                        result = i as isize;
                        break;
                    }
                }
                result
            };
            if search == -1 {
                break;
            }

            let mut cmp1 = (search + 1) as usize;
            let mut cmp2 = pos + 1;
            while cmp2 < cmp_end && buffer[cmp1] == buffer[cmp2] {
                cmp1 += 1;
                cmp2 += 1;
            }
            let len_ = cmp2 - pos;

            if found_len < len_ {
                found_len = len_;
                found = search as usize;
                if found_len == max_len {
                    break;
                }
            }
            search += 1
        }
    }
    (found, found_len)
}

/// most likely broken, but it "should" compress your stuff,
/// I recommend using the other Yaz0 library for compression.
/// The one who wrote it seems to be a lot more knowledgable
/// in both Rust and Compression algorithms :/
pub fn compress(buffer: Vec<u8>, level: usize) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
    let search_range = {
        if level == 0 {
            0
        } else if level < 9 {
            0x10E0 * level / 9 - 0x0E0
        } else {
            0x1000
        }
    };

    let src_end = buffer.len();
    let max_len = 0x111;
    let mut code_byte_pos;
    let mut pos = 0;

    while pos < src_end {
        code_byte_pos = result.len();
        result.push(0);
        for i in 0..8 {
            if pos >= src_end {
                break;
            }
            let mut found_len = 1;
            let mut found = 0;
            if search_range > 0 {
                let ret = compression_search(
                    &buffer, pos, max_len, search_range, src_end
                );
                found = ret.0;
                found_len = ret.1;
            }
            if found_len > 2 {
                let delta = pos - found - 1;

                if found_len < 0x12 {
                    result.push((delta >> 8 | (found_len - 2) << 4) as u8);
                    result.push((delta & 0xFF) as u8);
                } else {
                    result.push((delta >> 8) as u8);
                    result.push((delta & 0xFF) as u8);
                    result.push(((found_len - 0x12) & 0xFF) as u8);
                }
                pos += found_len;

            } else {
                result[code_byte_pos] |= 1 << (7 - i);
                result.push(buffer[pos]);
                pos += 1;
            }
        }
    }
    result
}


// aliases for compress and decompress:
/// another name for compressing
pub fn deflate(buffer: Vec<u8>, level: usize) -> Vec<u8> {
    compress(buffer, level)
}
/// another name for decompressing
pub fn inflate(buffer: Vec<u8>) -> Vec<u8> {
    decompress(buffer)
}

/// generates a Yaz0 header.
pub fn generate_header(size: u32) -> Vec<u8> { // TODO: make it a slice ?
    let mut result: Vec<u8> = b"Yaz0".to_vec();
    result.write_u32::<BE>(size).unwrap();
    result.append(&mut [0u8; 8].to_vec());
    result
}

fn get_size(buffer: &Vec<u8>) -> usize {
    // TODO: looks pretty slow :/
    let mut rdr = Cursor::new(buffer);
    rdr.seek(SeekFrom::Start(4)).unwrap();
    rdr.read_u32::<BE>().unwrap() as usize
}

// for more information please visit the link mentioned on top of this file
/// decompresses the entire buffer :)
pub fn decompress(buffer: Vec<u8>) -> Vec<u8> {
    let size = get_size(&buffer);
    let mut result: Vec<u8> = Vec::with_capacity(size);
    let mut cursor = 16; // first 16 bytes are the header so we start reading at byte 17
    let mut rcursor = 0; // result cursor
    let mut code = buffer[cursor]; cursor += 1; // code tells what to do next...
    let mut used_bits = 0;

    while rcursor < size {
        if used_bits == 8 {
            code = buffer[cursor]; cursor += 1;
            used_bits = 0;
        }

        // simply copy the byte:
        if 0x80 & code != 0 {
            result.push(buffer[cursor]); rcursor += 1;
            cursor += 1;
        } else {
            let mut byte_count = buffer[cursor] as usize; cursor += 1;
            let b2 = buffer[cursor] as usize; cursor += 1;
            // copy_source = where the byte is located in the source buffer
            let mut copy_source = rcursor - ((byte_count & 0x0F) << 8 | b2) - 1;
            if byte_count >> 4 == 0 {
                byte_count = (buffer[cursor] as usize) + 0x12; cursor += 1;
            } else {
                byte_count = (byte_count >> 4) + 2;
            }
            for _ in 0..byte_count {
                result.push(result[copy_source]);
                copy_source += 1;
                rcursor += 1;
            }

        }
        code <<= 1;
        used_bits += 1;
    }
    result
}

/// my first try porting a compression algorithm,
/// it's really just the direct port of the python3
/// version ;)
///
/// just call it with your buffer and it will give you a decompressed buffer back.
pub fn alt_decompress(buffer: Vec<u8>) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    let src_end: usize = buffer.len();
    let dest_end: usize = {
        let mut rdr = Cursor::new(get_subvector(&buffer, 4, 8));
        rdr.read_u32::<BE>().unwrap() as usize
    };

    // fill vector with 0's to prevent indexing error :)
    // NOTE is pointless ._.
    while result.len() != dest_end {
        result.push(0);
    }

    let mut code = buffer[16];

    let mut src_pos: usize = 17;
    let mut dest_pos: usize = 0;

    while src_pos < src_end && dest_pos < dest_end {
        let mut normal_exit: bool = true;
        for _ in 0..8 {
            if src_pos >= src_end || dest_pos >= dest_end {
                normal_exit = false;
                break;
            }

            if code & 0x80 > 0 {
                result[dest_pos] = buffer[src_pos];
                dest_pos += 1;
                src_pos += 1;
            } else {
                let b1 = buffer[src_pos] as usize; src_pos += 1;
                let b2 = buffer[src_pos] as usize; src_pos += 1;
                let mut copy_src: usize = {
                    dest_pos - ((b1 & 0x0F) << 8 | b2) - 1
                };
                let n = {
                    if b1 >> 4 == 0 {
                        let temp: usize = (buffer[src_pos] as usize) + 0x12;
                        src_pos += 1;
                        temp
                    } else {
                        ((b1 >> 4) + 2) as usize
                    }
                };

                for _ in 0..n {
                    result[dest_pos] = result[copy_src];
                    dest_pos += 1;
                    copy_src += 1;
                }
            }
            code <<= 1;
        }
        if normal_exit {
            if src_pos >= src_end || dest_pos >= dest_end {
                break;
            }
            code = buffer[src_pos];
            src_pos += 1;
        }
    }
    result
}
