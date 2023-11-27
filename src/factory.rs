use std::{
    io::{self, Read, Write},
    fs::{File, self},
    time::{SystemTime, UNIX_EPOCH, Duration},
    path::PathBuf, env, cmp::Ordering, sync::{Arc, Mutex, mpsc}, thread,
};

use rayon::prelude::*;
use rand::Rng;
use serde::{Serialize, ser::SerializeMap};
use serde::ser::SerializeSeq;
use flate2::{ Compression, write::GzEncoder };

use crate::AppState;

#[derive(Debug, Clone, Copy)]
pub struct Keyboard {
    keys: [Key; 47],
    pub heatmap: [f32; 47],
    hands: [Finger; 8],
}

impl Serialize for Keyboard {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let mut seq = serializer.serialize_seq(Some(self.keys.len()))?;
        for k in self.keys {
            seq.serialize_element(&k)?;
        }
        seq.end()
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize)]
enum Finger {
    // row 0 (top row) through row 3 (bottom)
    LPinky([usize; 2], usize, usize, usize),
    LRing(usize, usize, usize, usize),
    LMid(usize, usize, usize, usize),
    LIndex([usize; 2], [usize; 2], [usize; 2], [usize; 2]),

    RIndex([usize; 2], [usize; 2], [usize; 2], [usize; 2]),
    RMid(usize, usize, usize, usize),
    RRing(usize, usize, usize, usize),
    RPinky([usize; 3], [usize; 4], [usize; 2], usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Letter(char, char),
    StaticLetter(char, char),
    Number(char, char),
    Punctuation(char, char),
}

impl Key {
    pub fn match_char(&self, c: char) -> bool {
        match self {
            Key::Letter(k1, k2) =>       if *k1 == c || *k2 == c { true } else { false },
            Key::StaticLetter(k1, k2) => if *k1 == c || *k2 == c { true } else { false },
            Key::Number(k1, k2) =>       if *k1 == c || *k2 == c { true } else { false },
            Key::Punctuation(k1, k2) =>  if *k1 == c || *k2 == c { true } else { false },
        }
    }
    pub fn key_val(&self) -> String {
        match self {
            Key::Letter(x, _) => x.to_string(),
            Key::StaticLetter(x, _) => x.to_string(),
            Key::Punctuation(x, _) => x.to_string(),
            Key::Number(x, _) => x.to_string(),
        }
    }
}

impl Serialize for Key {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer {
        match self {
            Key::Letter(lower, upper) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("lower", lower)?;
                map.serialize_entry("upper", upper)?;
                map.end()
            },
            Key::StaticLetter(lower, upper) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("lower", lower)?;
                map.serialize_entry("upper", upper)?;
                map.end()
            },
            Key::Number(lower, upper) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("lower", lower)?;
                map.serialize_entry("upper", upper)?;
                map.end()
            },
            Key::Punctuation(lower, upper) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("lower", lower)?;
                map.serialize_entry("upper", upper)?;
                map.end()
            },
        }
    }
}

impl PartialEq for Keyboard {
    fn eq(&self, other: &Self) -> bool {
        for (i, k) in self.keys.iter().enumerate() {
            if *k != other.keys[i] { return false }
        }
        true
    }

    fn ne(&self, other: &Self) -> bool {
        for (i, k) in self.keys.iter().enumerate() {
            if *k == other.keys[i] { return false }
        }
        true
        
    }
}

impl Keyboard {
    pub fn new_random() -> Self {
        let mut available_spots = vec![15, 16, 19, 20, 21, 22, 23, 24, 28, 30,
                                      35, 36, 39, 40, 41, 42, 43, 44, 45];
        let mut available_keys = vec![
            Key::Letter('i', 'I'),
            Key::Letter('o', 'O'),

            Key::Letter('f', 'F'),
            Key::Letter('n', 'N'),
            Key::Letter('w', 'W'),
            Key::Letter('g', 'G'),
            Key::Letter('q', 'Q'),
            Key::Letter('z', 'Z'),

            
            Key::Letter('b', 'B'),
            Key::Letter('m', 'M'),
            Key::Letter('x', 'X'),
            Key::Letter('u', 'U'),
            Key::Letter('d', 'D'),
            Key::Letter('p', 'P'),
            Key::Letter('v', 'V'),

            Key::Letter('r', 'R'),
            Key::Letter('t', 'T'),
            Key::Letter('c', 'C'),

            Key::Letter('y', 'Y'),
        ];


        let mut keys: [Option<Key>; 47] = [None; 47];

        keys[0] = Some(Key::Punctuation('`', '~'));
        keys[1] = Some(Key::Number('1', '!'));
        keys[2] = Some(Key::Number('2', '@'));
        keys[3] = Some(Key::Number('3', '#'));
        keys[4] = Some(Key::Number('4', '$'));
        keys[5] = Some(Key::Number('5', '%'));
        keys[6] = Some(Key::Number('6', '^'));
        keys[7] = Some(Key::Number('7', '&'));
        keys[8] = Some(Key::Number('8', '*'));
        keys[9] = Some(Key::Number('9', '('));
        keys[10] = Some(Key::Number('0', ')'));
        keys[11] = Some(Key::Punctuation(',', '<'));
        keys[12] = Some(Key::Punctuation('.', '>'));

        keys[13] = Some(Key::Punctuation('[', '{'));
        keys[14] = Some(Key::Punctuation(']', '}'));
        keys[17] = Some(Key::Punctuation('-', '_'));
        keys[18] = Some(Key::Punctuation('=', '+'));
        keys[25] = Some(Key::Punctuation('\\', '|'));

        keys[26] = Some(Key::Letter('a', 'A'));
        keys[27] = Some(Key::Letter('s', 'S'));
        keys[29] = Some(Key::Letter('e', 'E'));
        keys[31] = Some(Key::StaticLetter('h', 'H'));
        keys[32] = Some(Key::StaticLetter('j', 'J'));
        keys[33] = Some(Key::StaticLetter('k', 'K'));
        keys[34] = Some(Key::StaticLetter('l', 'L'));


        keys[37] = Some(Key::Punctuation(';', ';'));
        keys[38] = Some(Key::Punctuation('\'', '"'));
        keys[46] = Some(Key::Punctuation('/', '?'));

        available_spots.sort_by(|_, _|
            if rand::thread_rng().gen_bool(0.5) { Ordering::Greater } else { Ordering::Less });

        available_keys.sort_by(|_, _|
            if rand::thread_rng().gen_bool(0.5) { Ordering::Greater } else { Ordering::Less });

        for &spot in available_spots.iter() {
            keys[spot] = available_keys.pop();
        }

        let key_vec: Vec<Key> = keys
            .iter()
            .map(|&k| k.unwrap())
            .collect();


        use Finger as F;
        Self {
            keys: key_vec.try_into().unwrap(),
            heatmap: [
3.,     2.,     2.,     2.,     2.,     3.,     3.,     2.,     2.,     2.,     2.,     3.,     4.,

            1.25,   1.5,    0.75,   0.75,   2.,     2.5,    0.75,   0.75,   1.,     1.,     3.,     3.5,    4.,

              0.25,   0.5,    0.,     0.,     1.,     1.,     0.,     0.,     0.5,    0.25,   1.,

                  1.25,   1.5,    1.,     1.,     2.,     1.,     1.,     1.,     1.5,    1.25,
            ],
            hands: [
                F::LPinky(
                    [0, 1],
                    13,
                    26,
                    37,
                ),
                F::LRing(
                    2,
                    14,
                    27,
                    38,
                ),
                F::LMid(
                    3,
                    15,
                    28,
                    39,
                ),
                F::LIndex(
                    [4, 5],
                    [16, 17],
                    [29, 30],
                    [40, 41],
                ),

                F::RIndex(
                    [6, 7],
                    [18, 19],
                    [31, 32],
                    [42, 43]
                ),
                F::RMid(
                    8,
                    20,
                    33,
                    44,
                ),
                F::RRing(
                    9,
                    21,
                    34,
                    45,
                ),
                F::RPinky(
                    [10, 11, 12],
                    [22, 23, 24, 25],
                    [35, 36],
                    46,
                ),
            ] 
        }
    }

    pub fn print_self(&self) {
        // `   1   2   3   4   5   6   7   8   9   0   -   =
        //       q   w   e   r   t   y   u   i   o   p   [   ]   \
        //        a   s   d   f   g   h   j   k   l   ;   '
        //          z   x   c   v   b   n   m   ,   .   /
        println!("{}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}",
            self.keys[0].key_val(),
            self.keys[1].key_val(),
            self.keys[2].key_val(),
            self.keys[3].key_val(),
            self.keys[4].key_val(),
            self.keys[5].key_val(),
            self.keys[6].key_val(),
            self.keys[7].key_val(),
            self.keys[8].key_val(),
            self.keys[9].key_val(),
            self.keys[10].key_val(),
            self.keys[11].key_val(),
            self.keys[12].key_val(),
        );
        println!("      {}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}",
            self.keys[13].key_val(),
            self.keys[14].key_val(),
            self.keys[15].key_val(),
            self.keys[16].key_val(),
            self.keys[17].key_val(),
            self.keys[18].key_val(),
            self.keys[19].key_val(),
            self.keys[20].key_val(),
            self.keys[21].key_val(),
            self.keys[22].key_val(),
            self.keys[23].key_val(),
            self.keys[24].key_val(),
            self.keys[25].key_val(),
        );
        println!("       {}   {}   {}   {}   {}   {}   {}   {}   {}   {}   {}",
            self.keys[26].key_val(),
            self.keys[27].key_val(),
            self.keys[28].key_val(),
            self.keys[29].key_val(),
            self.keys[30].key_val(),
            self.keys[31].key_val(),
            self.keys[32].key_val(),
            self.keys[33].key_val(),
            self.keys[34].key_val(),
            self.keys[35].key_val(),
            self.keys[36].key_val(),
        );
        println!("         {}   {}   {}   {}   {}   {}   {}   {}   {}   {}",
            self.keys[37].key_val(),
            self.keys[38].key_val(),
            self.keys[39].key_val(),
            self.keys[40].key_val(),
            self.keys[41].key_val(),
            self.keys[42].key_val(),
            self.keys[43].key_val(),
            self.keys[44].key_val(),
            self.keys[45].key_val(),
            self.keys[46].key_val(),
        );
    }
    pub fn reproduce(&self, mutations: usize) -> Keyboard {
        let mut new_keyboard = self.clone();

        let available_keys = [15, 16, 19, 20, 21, 22, 23, 24, 28, 30, 35, 36, 39, 40, 41, 42, 43, 44, 45];

        let letter_only_keys = [19, 20];

        let non_letter_only_keys = [21, 22, 23, 24, 36, 45];

        let letter_keys: Vec<usize> = available_keys
            .into_iter()
            .filter(|&k| {
                match new_keyboard.keys[k] {
                    Key::Letter(_, _) => true,
                    _ => false,
                }
            })
            .collect();

        let punc_keys: Vec<usize> = available_keys
            .into_iter()
            .filter(|&k| {
                match new_keyboard.keys[k] {
                    Key::Punctuation(_, _) => true,
                    _ => false,
                }
            })
            .collect();

        for _ in 0..mutations {
            let rand_key_index = available_keys[rand::thread_rng().gen_range(0..available_keys.len())];
            let rand_key_punc = punc_keys
                .clone()
                .into_iter()
                .find(|i| *i == rand_key_index)
                .is_some();
            let rand_key_letter_only = letter_only_keys
                .clone()
                .into_iter()
                .find(|i| *i == rand_key_index)
                .is_some();

            let other_key_index = match (rand_key_letter_only, rand_key_punc) {
                (true, _)      => letter_keys[rand::thread_rng().gen_range(0..letter_keys.len())],
                (false, true)  => non_letter_only_keys[rand::thread_rng().gen_range(0..non_letter_only_keys.len())],
                (false, false) => available_keys[rand::thread_rng().gen_range(0..available_keys.len())],
            };

            let key1 = new_keyboard.keys[rand_key_index].clone();
            let key2 = new_keyboard.keys[other_key_index].clone();

            {
                let key1_ref = new_keyboard.keys.get_mut(rand_key_index).unwrap();
                *key1_ref = key2;
            }
            {
                let key2_ref = new_keyboard.keys.get_mut(other_key_index).unwrap();
                *key2_ref = key1;
            }
        }

        new_keyboard
    }

    pub fn new_47() -> Self {
        use Finger as F;
        Self {
            keys: [
                Key::Punctuation('`', '~'),
                Key::Number('1', '!'),
                Key::Number('2', '@'),
                Key::Number('3', '#'),
                Key::Number('4', '$'),
                Key::Number('5', '%'),
                Key::Number('6', '^'),
                Key::Number('7', '&'),
                Key::Number('8', '*'),
                Key::Number('9', '('),
                Key::Number('0', ')'),
                Key::Punctuation(',', '<'),
                Key::Punctuation('.', '>'),

                Key::Punctuation('[', '{'),
                Key::Punctuation(']', '}'),
                Key::Letter('i', 'I'),
                Key::Letter('o', 'O'),
                Key::Punctuation('-', '_'),
                Key::Punctuation('=', '+'),
                Key::Letter('f', 'F'),
                Key::Letter('n', 'N'),
                Key::Letter('w', 'W'),
                Key::Letter('v', 'V'),
                Key::Letter('q', 'Q'),
                Key::Letter('z', 'Z'),
                Key::Punctuation('\\', '|'),

                Key::Letter('a', 'A'),
                Key::Letter('r', 'R'),
                Key::Letter('t', 'T'),
                Key::Letter('e', 'E'),
                Key::Letter('c', 'c'),
                Key::StaticLetter('h', 'H'),
                Key::StaticLetter('j', 'J'),
                Key::StaticLetter('k', 'K'),
                Key::StaticLetter('l', 'L'),
                Key::Letter('s', 'S'),
                Key::Letter('g', 'G'),
                
                Key::Punctuation(';', ':'),
                Key::Punctuation('\'', '"'),
                Key::Letter('b', 'B'),
                Key::Letter('m', 'M'),
                Key::Letter('x', 'X'),
                Key::Letter('u', 'U'),
                Key::Letter('d', 'D'),
                Key::Letter('p', 'P'),
                Key::Letter('y', 'Y'),
                Key::Punctuation('/', '?'),
            ],
            heatmap: [
3.,     2.,     2.,     2.,     2.,     3.,     3.,     2.,     2.,     2.,     2.,     3.,     4.,

            1.25,   1.5,    0.75,   0.75,   2.,     2.5,    0.75,   0.75,   1.,     1.,     3.,     3.5,    4.,

              0.25,   0.5,    0.,     0.,     1.,     1.,     0.,     0.,     0.5,    0.25,   1.,

                  1.25,   1.5,    1.,     1.,     2.,     1.,     1.,     1.,     1.5,    1.25,
            ],
            hands: [
                F::LPinky(
                    [0, 1],
                    13,
                    26,
                    37,
                ),
                F::LRing(
                    2,
                    14,
                    27,
                    38,
                ),
                F::LMid(
                    3,
                    15,
                    28,
                    39,
                ),
                F::LIndex(
                    [4, 5],
                    [16, 17],
                    [29, 30],
                    [40, 41],
                ),

                F::RIndex(
                    [6, 7],
                    [18, 19],
                    [31, 32],
                    [42, 43]
                ),
                F::RMid(
                    8,
                    20,
                    33,
                    44,
                ),
                F::RRing(
                    9,
                    21,
                    34,
                    45,
                ),
                F::RPinky(
                    [10, 11, 12],
                    [22, 23, 24, 25],
                    [35, 36],
                    46,
                ),
            ] 
        }
    }

    pub fn get_key(&self, c: char) -> Option<&Key> {
        let mut rv = None;
        for key in self.keys.iter() {
            if key.match_char(c) {
                rv = Some(key);
                break;
            }
        }
        rv
    }

    pub fn index_to_row(&self, index: usize) -> Option<usize> {
        if index < 13 { Some(0) }
        else if index >= 13 && index < 26 { Some(1) }
        else if index >= 26 && index < 37 { Some(2) }
        else if index >= 37 && index < 47 { Some(3) }
        else { None }
    }

    pub fn distance(&self, a: usize, b: usize) -> f32 {
        if a == b { return 0. };

        let a_hand = self.which_hand(a).expect("a_hand error");
        let b_hand = self.which_hand(b).expect("b_hand error");

        let a_row = self.index_to_row(a).expect("a_row error");
        let b_row = self.index_to_row(b).expect("b_row error");

        let row_diff = b_row as i8 - a_row as i8;

        if a_hand == b_hand {
            if row_diff == 0 { self.heatmap[b] as f32 * 0.75}
            else if row_diff < 0 { self.heatmap[b] as f32 * 1.5 }
            else { self.heatmap[b] as f32 }
        } else {
            self.heatmap[b] as f32 * 1.25
        }
    }

    pub fn char_to_index(&self, c: char) -> Option<usize> {
        for (i, k) in self.keys.iter().enumerate() {
            if k.match_char(c) {
                return Some(i.try_into().unwrap());
            }
        }
        None
    }

    pub fn which_hand(&self, index: usize) -> Option<Hand> {
        let mut i = 0;
        loop {
            let hand = if i < 4 { Hand::Left } else { Hand::Right };
            if let Some(finger) = self.hands.get(i) {
                if finger.is_inside(index).is_some() { break Some(hand) }
                i += 1;
            } else {
                break None
            }
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

impl Finger {
    pub fn is_inside(&self, key: usize) -> Option<usize> {
        // returns the row if true
        match self {
            Self::LPinky(zero, one, two, three) => {
                let mut rv = None;
                for k in zero {
                    if *k == key { 
                        rv = Some(0)
                    }
                }
                if key == *one { rv = Some(1) }
                else if key == *two { rv = Some(2) }
                else if key == *three { rv = Some(3) }
                rv
            },
            Self::LRing(zero, one, two, three) => {
                if key == *zero { Some(1) }
                else if key == *one { Some(1) }
                else if key == *two { Some(2) }
                else if key == *three { Some(3) }
                else { None }
            },
            Self::LMid(zero, one, two, three) => {
                if key == *zero { Some(1) }
                else if key == *one { Some(1) }
                else if key == *two { Some(2) }
                else if key == *three { Some(3) }
                else { None }
            },
            Self::LIndex(zero, one, two, three) => {
                let mut rv = None;
                for k in zero {
                    if *k == key { 
                        rv = Some(0)
                    }
                }
                for k in one {
                    if *k == key { 
                        rv = Some(1)
                    }
                }
                for k in two {
                    if *k == key { 
                        rv = Some(2)
                    }
                }
                for k in three {
                    if *k == key { 
                        rv = Some(3)
                    }
                }
                rv
            },

            Self::RIndex(zero, one, two, three) => {
                let mut rv = None;
                for k in zero {
                    if *k == key { 
                        rv = Some(0)
                    }
                }
                for k in one {
                    if *k == key { 
                        rv = Some(1)
                    }
                }
                for k in two {
                    if *k == key {
                        rv = Some(2)
                    }
                }
                for k in three {
                    if *k == key { 
                        rv = Some(3)
                    }
                }
                rv
            },
            Self::RMid(zero, one, two, three) => {
                if key == *zero { Some(1) }
                else if key == *one { Some(1) }
                else if key == *two { Some(2) }
                else if key == *three { Some(3) }
                else { None }
            },
            Self::RRing(zero, one, two, three) => {
                if key == *zero { Some(1) }
                else if key == *one { Some(1) }
                else if key == *two { Some(2) }
                else if key == *three { Some(3) }
                else { None }
            },
            Self::RPinky(zero, one, two, three) => {
                let mut rv = None;
                for k in zero {
                    if *k == key { 
                        rv = Some(0)
                    }
                }
                for k in one {
                    if *k == key { 
                        rv = Some(1)
                    }
                }
                for k in two {
                    if *k == key { 
                        rv = Some(2)
                    }
                }
                if key == *three { rv = Some(3) }
                rv
            },
        }
    }
}


fn read_file(path: PathBuf) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents)
}

fn read_dir(path: PathBuf, keyboard: &Keyboard) -> io::Result<f32> {
    let score = atomic_float::AtomicF32::new(0.);

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        if entry_path.is_file() {
            if let Ok(contents) = read_file(entry_path) {
                contents
                    .chars()
                    .collect::<Vec<char>>()
                    .windows(2)
                    .for_each(|c| {
                        match (keyboard.char_to_index(c[0]), keyboard.char_to_index(c[1])) {
                            (Some(a), Some(b)) => {
                                score.fetch_add(keyboard.distance(a, b), std::sync::atomic::Ordering::Relaxed);
                            },
                            (None, Some(b)) => {
                                score.fetch_add(keyboard.heatmap[b] as f32, std::sync::atomic::Ordering::Relaxed);
                            },
                            (_, None) => (),
                        };
                    }
                )
            }
        } else if entry_path.is_dir() {
            score.fetch_add(read_dir(entry_path, keyboard).unwrap(), std::sync::atomic::Ordering::Relaxed);
        }
    }

    Ok(score.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn start_generation(job_name: String, batch_size: usize, shared_state: Arc<Mutex<AppState>>) {
    let mut path = env::current_dir()
        .expect("lmao");
    path.push("pile");

    let (tx, rx) = mpsc::channel::<String>();

    thread::spawn(move || {
        let mut url = env::var("URL")
            .expect("Missing Url");
        url.push_str("/api/v2/write");

        let client = reqwest::blocking::Client::new();

        let mut dp_buffer = String::new();
        let mut count = 0;
        let mut last_batch = SystemTime::now();
        for recv in rx {
            dp_buffer += (recv + "\n").as_str();

            let msg_limit = count % 5000 == 0;
            let time_limit = SystemTime::now().duration_since(last_batch).unwrap() > Duration::new(5, 0);
            if msg_limit || time_limit {
                let mut e = GzEncoder::new(Vec::new(), Compression::default());
                e.write_all(dp_buffer.as_bytes()).unwrap();

                client
                    .post(url.clone())
                    .query(&[("bucket", "keyboard_gen"), ("precision", "ms")])
                    .header("Authorization", format!("Token {}", env::var("KEY").unwrap()))
                    .header("Content-Type", "text/plain; charset=utf8")
                    .header("Content-Encoding", "gzip")
                    .header("Accept", "application/json")
                    .body(e.finish().unwrap())
                    .send()
                    .unwrap();

                dp_buffer = String::new();
                count = 0;
                last_batch = SystemTime::now();
            }

            count += 1;
        }

        // flush remaining
        client
            .post(url.clone())
            .query(&[("bucket", "keyboard_gen"), ("precision", "ms")])
            .header("Authorization", format!("Token {}", env::var("KEY").unwrap()))
            .header("Content-Type", "text/plain; charset=utf8")
            .header("Accept", "application/json")
            .body(dp_buffer.clone())
            .send()
            .unwrap()
    });

    let send_msg = |tags: Vec<(String, String)>, fields: Vec<(String, String)>| {
        tx.send(format!(
        "{},{} {} {}",
        job_name,
        tags.iter()
            .map(|(key, val)| {
                format!("{}={}",
                        str::replace(key, " ", "\\ "),
                        str::replace(val, " ", "\\ "))
            })
            .collect::<Vec<String>>()
            .join(","),
        fields.iter()
            .map(|(key, val)| {
                format!("{}=\"{}\"",
                        str::replace(key, " ", "\\ "),
                        str::replace(val, " ", "\\ "))
            })
            .collect::<Vec<String>>()
            .join(","),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
        )).unwrap();
    };

    println!("start keyboards");
    let mut results: Vec<(f32, Keyboard)> = (0..batch_size)
        .into_par_iter()
        .map(|i| {
            send_msg(vec![("keyboard".into(), i.to_string())],
                     vec![("status".into(), "preparation".into())]);

            let mut keyboards: Vec<Keyboard> = vec![Keyboard::new_random(); 100];
            let mut top_50 = (0..50)
                .into_par_iter()
                .map(|_| {
                    let k = Keyboard::new_random();
                    (read_dir(path.clone(), &k).expect("you fucked up"), k)
                })
                .collect::<Vec<(f32, Keyboard)>>();
                
            send_msg(
                vec![("keyboard".into(), i.to_string())],
                vec![("status".into(), "start".into())]
            );
            println!("keyboard {}", i);

            let mut score_history: [f32; 100] = [10000000000.; 100];
            let mut generation_count = 0_usize;
            let result = loop {
                send_msg(
                    vec![("keyboard".into(), i.to_string()), ("generation".into(), generation_count.to_string())],
                    vec![("status".into(), "start".into())]
                );
                let mut result = keyboards
                    .into_par_iter()
                    .map(|keyboard| {
                        // TODO: generate uuid
                        if let Some(entry) = top_50.iter()
                                                   .find(|(_, k_cmp)| *k_cmp == keyboard) {
                            entry.clone()
                        } else {
                           (read_dir(path.clone(), &keyboard).expect("you fucked up again"), keyboard.clone())
                        }
                    })
                    .collect::<Vec<(f32, Keyboard)>>();

                result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                keyboards = result.iter().map(|(_, k)| k.clone()).collect();
                top_50 = result.iter()
                    .enumerate()
                    .filter_map(|(i, data)| {
                        if i < 50 { Some(data.clone()) }
                        else { None }
                    })
                    .collect();

                for (i, (_, keyboard)) in top_50.iter().enumerate() {
                    let k = keyboards.get_mut(i + 50).expect("why did you do that?");
                    *k = keyboard.reproduce(match i % 6 {
                            0 => 1,
                            1 => 2,
                            2 => 4,
                            3 => 8,
                            4 => 16,
                            5 => 32,
                            _ => panic!()
                        });
                }

                send_msg(
                    vec![("keyboard".into(), i.to_string()), ("generation".into(), generation_count.to_string())],
                    vec![("status".into(), "end".into())]
                );

                generation_count += 1;

                score_history[generation_count % 100] = top_50[0].0;
                if score_history.iter().all(|&s| s == score_history[0]) {
                    let mut state = shared_state.lock().unwrap();
                    state.add_one_completed();
                    break top_50[0];
                }
            };

            send_msg(
                vec![("keyboard".into(), i.to_string())],
                vec![("status".into(), "end".into())]
            );

            result
        })
        .collect();

    results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut state = shared_state.lock().unwrap();
    *state = AppState::Completed(results);
}
