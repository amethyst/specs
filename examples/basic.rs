extern crate parsec;

#[derive(Clone, Debug)]
struct CompInt(i8);
impl parsec::Component for CompInt {
    type Storage = parsec::VecStorage<CompInt>;
}
#[derive(Clone, Debug)]
struct CompBool(bool);
impl parsec::Component for CompBool {
    type Storage = parsec::VecStorage<CompBool>;
}

fn main() {
    let mut w = parsec::World::new();
    w.register::<CompInt>();
    w.register::<CompBool>();
    let mut scheduler = parsec::Scheduler::new(w, 4);
    scheduler.add_entity().with(CompInt(4)).with(CompBool(false)).build();
    scheduler.add_entity().with(CompInt(-1)).with(CompBool(false)).build();

    scheduler.run1w1r(|b: &mut CompBool, a: &CompInt| {
        b.0 = a.0 > 0;
    });
    scheduler.run(|warg| {
        let (mut sa, sb, entities) = warg.fetch(|comp| {
            (comp.write::<CompInt>(),
             comp.read::<CompBool>(),
             comp.entities())
        });

        //println!("{:?} {:?}", &*sa, &*sb);
        for &ent in entities.iter() {
            use parsec::Storage;
            if let (Some(a), Some(b)) = (sa.get_mut(ent), sb.get(ent)) {
                a.0 = if b.0 {2} else {0};
            }
        }
    });
    scheduler.run0w2r(|a: &CompInt, b: &CompBool| {
        println!("Entity {} {}", a.0, b.0);
    });
    scheduler.wait();
}
