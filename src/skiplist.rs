use std::cmp::Ordering;
use std::fmt;
use std::ptr;

const MAX_LEVEL: usize = 16;

struct Node {
    score: f64,
    member: Vec<u8>,
    forward: Vec<*mut Node>,
}

impl Node {
    fn new(score: f64, member: Vec<u8>, level: usize) -> *mut Node {
        let node = Box::new(Node {
            score,
            member,
            forward: vec![ptr::null_mut(); level],
        });
        Box::into_raw(node)
    }
}

fn compare_nodes(score1: f64, member1: &[u8], score2: f64, member2: &[u8]) -> Ordering {
    match score1.partial_cmp(&score2) {
        Some(Ordering::Equal) | None => member1.cmp(member2),
        Some(ord) => ord,
    }
}

pub struct SkipList {
    head: *mut Node,
    level: usize,
    length: usize,
}

unsafe impl Send for SkipList {}
unsafe impl Sync for SkipList {}

impl fmt::Debug for SkipList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SkipList(len={})", self.length)
    }
}

impl PartialEq for SkipList {
    fn eq(&self, other: &Self) -> bool {
        if self.length != other.length {
            return false;
        }
        self.get_range(0, self.length) == other.get_range(0, other.length)
    }
}

impl Clone for SkipList {
    fn clone(&self) -> Self {
        let mut new_sl = SkipList::new();
        for (member, score) in self.get_range(0, self.length) {
            new_sl.insert(score, member);
        }
        new_sl
    }
}

impl SkipList {
    pub fn new() -> Self {
        let head = Node::new(0.0, Vec::new(), MAX_LEVEL);
        SkipList {
            head,
            level: 1,
            length: 0,
        }
    }

    fn random_level(&self) -> usize {
        let mut lvl = 1;
        while lvl < MAX_LEVEL && (rand_simple() & 0xFFFF) < (0.25 * 65536.0) as u32 {
            lvl += 1;
        }
        lvl
    }

    pub fn insert(&mut self, score: f64, member: Vec<u8>) -> bool {
        let mut update = [self.head; MAX_LEVEL];
        let mut curr = self.head;

        for i in (0..self.level).rev() {
            unsafe {
                while !(&(*curr).forward)[i].is_null() {
                    let next = (&(*curr).forward)[i];
                    if compare_nodes((&(*next)).score, &(&(*next)).member, score, &member) == Ordering::Less {
                        curr = next;
                    } else {
                        break;
                    }
                }
            }
            update[i] = curr;
        }

        unsafe {
            let next = (&(*curr).forward)[0];
            if !next.is_null()
                && (&(*next)).score == score
                && (&(*next)).member == member
            {
                return false;
            }
        }

        let lvl = self.random_level();
        if lvl > self.level {
            for i in self.level..lvl {
                update[i] = self.head;
            }
            self.level = lvl;
        }

        let new_node = Node::new(score, member, lvl);
        for i in 0..lvl {
            unsafe {
                let target_forward = (&(*update[i]).forward)[i];
                (&mut (*new_node).forward)[i] = target_forward;
                (&mut (*update[i]).forward)[i] = new_node;
            }
        }

        self.length += 1;
        true
    }

    pub fn remove(&mut self, score: f64, member: &[u8]) -> bool {
        let mut update = [self.head; MAX_LEVEL];
        let mut curr = self.head;

        for i in (0..self.level).rev() {
            unsafe {
                while !(&(*curr).forward)[i].is_null() {
                    let next = (&(*curr).forward)[i];
                    if compare_nodes((&(*next)).score, &(&(*next)).member, score, member) == Ordering::Less {
                        curr = next;
                    } else {
                        break;
                    }
                }
            }
            update[i] = curr;
        }

        unsafe {
            let target = (&(*curr).forward)[0];
            if target.is_null() || (&(*target)).score != score || (&(*target)).member != member {
                return false;
            }

            for i in 0..self.level {
                if (&(*update[i]).forward)[i] == target {
                    (&mut (*update[i]).forward)[i] = (&(*target).forward)[i];
                }
            }

            let _ = Box::from_raw(target);

            while self.level > 1 && (&(*self.head).forward)[self.level - 1].is_null() {
                self.level -= 1;
            }

            self.length -= 1;
            true
        }
    }

    pub fn get_range(&self, start: usize, stop: usize) -> Vec<(Vec<u8>, f64)> {
        let mut result = Vec::new();
        if self.length == 0 || start > stop {
            return result;
        }

        let mut curr = unsafe { (&(*self.head).forward)[0] };
        let mut index = 0;

        while !curr.is_null() && index <= stop {
            if index >= start {
                unsafe {
                    result.push(((&(*curr)).member.clone(), (&(*curr)).score));
                }
            }
            curr = unsafe { (&(*curr).forward)[0] };
            index += 1;
        }

        result
    }

    pub fn len(&self) -> usize {
        self.length
    }
}

impl Drop for SkipList {
    fn drop(&mut self) {
        let mut curr = unsafe { (&(*self.head).forward)[0] };
        while !curr.is_null() {
            let next = unsafe { (&(*curr).forward)[0] };
            unsafe {
                let _ = Box::from_raw(curr);
            }
            curr = next;
        }
        unsafe {
            let _ = Box::from_raw(self.head);
        }
    }
}

static mut RAND_STATE: u64 = 88172645463325252;
fn rand_simple() -> u32 {
    unsafe {
        RAND_STATE = RAND_STATE.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (RAND_STATE >> 32) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skiplist_basic() {
        let mut sl = SkipList::new();
        assert_eq!(sl.len(), 0);

        sl.insert(10.0, b"a".to_vec());
        sl.insert(5.0, b"b".to_vec());
        sl.insert(15.0, b"c".to_vec());

        assert_eq!(sl.len(), 3);

        let range = sl.get_range(0, 2);
        assert_eq!(
            range,
            vec![
                (b"b".to_vec(), 5.0),
                (b"a".to_vec(), 10.0),
                (b"c".to_vec(), 15.0)
            ]
        );

        assert_eq!(sl.remove(10.0, b"a"), true);
        assert_eq!(sl.len(), 2);
    }
}
