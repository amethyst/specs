extern crate hibitset;
extern crate specs;

use hibitset::{BitSet, BitSetNot};
use specs::Join;

const COUNT: u32 = 100;

fn main() {
    let mut every3 = BitSet::new();
    for i in 0..COUNT {
        if i % 3 == 0 {
            every3.add(i);
        }
    }

    let mut every5 = BitSet::new();
    for i in 0..COUNT {
        if i % 5 == 0 {
            every5.add(i);
        }
    }

    // over engineered fizzbuzz because why not
    let mut list: Vec<String> = Vec::with_capacity(COUNT as usize);
    for id in 0..COUNT {
        list.push(format!("{}", id));
    }

    for (id, _) in (&BitSetNot(&every3), &every5).join() {
        list[id as usize] = format!("fizz {}", id);
    }

    for (id, _) in (&BitSetNot(&every5), &every3).join() {
        list[id as usize] = format!("buzz {}", id);
    }

    for (id, _) in (&every3, &every5).join() {
        list[id as usize] = format!("fizzbuzz {}", id);
    }

    println!("{:#?}", list);
}
