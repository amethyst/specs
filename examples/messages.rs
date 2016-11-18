extern crate specs;
use specs::{RunArg, System, MessageQueue};

#[derive(Clone, Debug)]
enum Message {
    Hello(String),
    Goodbye(String),
}

struct HelloSystem {}
impl System<Message, ()> for HelloSystem {
    fn run(&mut self, arg: RunArg, msg: MessageQueue<Message>, _: ()) {
        let _ = arg.fetch(|_|{});
        msg.send(Message::Hello("hey".to_owned()));
    }

    fn handle_message(&mut self, msg: &Message) {
        use Message::*;
        match *msg {
            Hello(_) => println!("Got a hello!"),
            Goodbye(_) => (),
        }
    }
}

struct GoodbyeSystem {}
impl System<Message, ()> for GoodbyeSystem {
    fn run(&mut self, arg: RunArg, msg: MessageQueue<Message>, _: ()) {
        let _ = arg.fetch(|_|{});
        msg.send(Message::Goodbye("bye".to_owned()));
    }

    fn handle_message(&mut self, msg: &Message) {
        use Message::*;
        match *msg {
            Hello(_) => (),
            Goodbye(_) => println!("Got a goodbye!"),
        }
    }
}

struct GreetzCounter {
    hello: u32,
    goodbye: u32,
}

impl System<Message, ()> for GreetzCounter {
    fn run(&mut self, arg: RunArg, _: MessageQueue<Message>, _: ()) {
        let _ = arg.fetch(|_|{});
        println!("I have seen {} hellos.", self.hello);
        println!("I have seen {} goodbyes.", self.goodbye);
    }

    fn handle_message(&mut self, msg: &Message) {
        use Message::*;
        match *msg {
            Hello(_) => self.hello += 1,
            Goodbye(_) => self.goodbye += 1,
        }
    }
}
fn main() {
    let mut planner = {
        let w = specs::World::new();
        specs::Planner::<Message,()>::new(w, 4)
    };
    let h = HelloSystem {};
    let g = GoodbyeSystem {};
    let ctr = GreetzCounter { hello: 0, goodbye: 0 };

    planner.add_system(h, "hello", 1);
    planner.add_system(g, "goodbye", 2);
    planner.add_system(ctr, "greetz", 3);

    for _ in 0..3 {
        planner.dispatch(());
        // technically not necessary; we call wait in handle_messages
        planner.wait();
        planner.handle_messages();
    }
}
