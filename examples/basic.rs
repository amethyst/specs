extern crate parsec;

#[derive(Clone, Debug)]
struct CompInt(i8);
impl parsec::Component for CompInt {
    type Storage = parsec::VecStorage<CompInt>;
}
#[derive(Clone, Debug)]
struct CompBool(bool);
impl parsec::Component for CompBool {
    type Storage = parsec::HashMapStorage<CompBool>;
}

fn main() {
    let (e, mut scheduler) = {
        let mut w = parsec::World::new();
        w.register::<CompInt>();
        w.register::<CompBool>();
        w.create_now().with(CompInt(4)).with(CompBool(false)).build();
        let e = w.create_now().with(CompInt(9)).with(CompBool(true)).build();
        w.create_now().with(CompInt(-1)).with(CompBool(false)).build();
        (e, parsec::Scheduler::new(w, 4))
    };

    scheduler.run1w1r(|b: &mut CompBool, a: &CompInt| {
        b.0 = a.0 > 0;
    });
    scheduler.world.delete_now(e);

    scheduler.run(|warg| {
        use parsec::Storage;
        let (mut sa, sb, entities) = warg.fetch(|w| {
            (w.write::<CompInt>(),
             w.read::<CompBool>(),
             w.entities())
        });

        //println!("{:?} {:?}", &*sa, &*sb);
        for ent in entities {
            use parsec::Storage;
            if let (Some(a), Some(b)) = (sa.get_mut(ent), sb.get(ent)) {
                a.0 = if b.0 {2} else {0};
            }
        }

        let e0 = warg.create();
        sa.insert(e0, CompInt(-4));
        let e1 = warg.create();
        sa.insert(e1, CompInt(-5));
        warg.delete(e0);
    });
    scheduler.run0w2r(|a: &CompInt, b: &CompBool| {
        println!("Entity {} {}", a.0, b.0);
    });

    scheduler.wait();
    if false {   // some debug output
        let w = &scheduler.world;
        println!("Generations: {:?}", &*w.get_generations());
        println!("{:?}", &*w.read::<CompInt>());
        println!("{:?}", &*w.read::<CompBool>());
        for e in w.entities() {
            println!("{:?}", e);
        }
    }
}
