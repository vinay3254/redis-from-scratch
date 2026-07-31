use crate::pubsub::PubSub;
use crate::resp::RespFrame;

pub fn publish(pubsub: &mut PubSub, args: &[Vec<u8>]) -> RespFrame {
    if args.len() != 2 {
        return RespFrame::Error("ERR wrong number of arguments for 'publish' command".into());
    }
    let count = pubsub.publish(&args[0], &args[1]);
    RespFrame::Integer(count as i64)
}
