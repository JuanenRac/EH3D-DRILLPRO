use eh3d_drillpro_protocol::positioner::Positioner;

#[test]
fn first_positive_move_takes_up_the_backlash() {
    let mut p = Positioner::new((8, 5));
    assert_eq!(p.plan((100, 50)), (108, 55));
}

#[test]
fn same_direction_adds_nothing() {
    let mut p = Positioner::new((8, 5));
    let t = (100, 50);
    p.plan(t);
    p.commit(t, (0, 0));
    assert_eq!(p.plan((200, 90)), (100, 40));
}

#[test]
fn reversal_adds_backlash_once_and_logical_position_ignores_it() {
    let mut p = Positioner::new((8, 5));
    let t1 = (100, 100);
    p.plan(t1);
    p.commit(t1, (0, 0));
    let back = p.plan((60, 100));
    assert_eq!(back, (-48, 0), "X reverses (-40 - 8), Y does not move");
    p.commit((60, 100), (0, 0));
    assert_eq!(p.pos, (60, 100));
    assert_eq!(p.plan((20, 100)), (-40, 0), "still going negative");
}

#[test]
fn zero_moves_do_not_flip_the_direction() {
    let mut p = Positioner::new((8, 5));
    assert_eq!(p.plan((0, 0)), (0, 0));
    assert_eq!(p.plan((10, 0)), (18, 0));
}

#[test]
fn an_interrupted_move_keeps_the_remaining_distance() {
    let mut p = Positioner::new((0, 0));
    let t = (100, 0);
    p.plan(t);
    p.commit(t, (30, 0));
    assert_eq!(p.pos, (70, 0));
}

#[test]
fn homing_resets_the_origin_and_direction() {
    let mut p = Positioner::new((8, 5));
    let t = (50, 50);
    p.plan(t);
    p.commit(t, (0, 0));
    p.set_origin();
    assert_eq!(p.pos, (0, 0));
    assert_eq!(p.plan((10, 10)), (18, 15));
}
