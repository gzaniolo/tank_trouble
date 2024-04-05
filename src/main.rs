

// commands despawn

use bevy::
{ 
    input::{keyboard::{Key, KeyboardInput}, ButtonState}, 
    prelude::*, 
    reflect::List, 
    sprite::{MaterialMesh2dBundle, Mesh2dHandle}, 
    utils:: {
        hashbrown::HashMap,
        Duration,
    }
};

use core::f32::consts::*;
use rand::prelude::*;

use std::{collections::VecDeque, rc::Rc};

#[cfg(feature = "debug")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;


/* TODO for now hardcode all the sizes for simplicity. In the future, can 
  elaborate more regarding resizeability and scale. The minesweeper example
  does a good job of this
*/

// TODO some hardcoding for now, ideally everything would scale...
const WINDOW_SIZE: (f32,f32) = (800.0, 600.0);

const ARENA_DIM: (usize, usize) = (12,9);

const BACKGROUND_COLOR: Color = Color::WHITE;

const WALL_COLOR: Color = Color::BLACK;
// TODO needs to change for resizeable screen
const WALL_SIZE: Vec3 = Vec3::new(WINDOW_SIZE.0 / (ARENA_DIM.0 as f32),5.0,0.0);

// TODO this is specific to the implementation of the board generation and 
//  would ideally be isolated, the implementation of the board generation
//  does not matter for the main file
const WALL_THRESHOLD: f32 = 0.5;

const TANK_COUNT: usize = 2;

const TANK1_COLOR: Color = Color::RED;
const TANK1_FWD_KEY: KeyCode = KeyCode::ArrowUp;
const TANK1_BWD_KEY: KeyCode = KeyCode::ArrowDown;
const TANK1_RIGHT_KEY: KeyCode = KeyCode::ArrowRight;
const TANK1_LEFT_KEY: KeyCode = KeyCode::ArrowLeft;
const TANK1_SHOOT_KEY: KeyCode = KeyCode::Space;

const TANK2_COLOR: Color = Color::GREEN;
const TANK2_FWD_KEY: KeyCode = KeyCode::KeyE;
const TANK2_BWD_KEY: KeyCode = KeyCode::KeyD;
const TANK2_RIGHT_KEY: KeyCode = KeyCode::KeyF;
const TANK2_LEFT_KEY: KeyCode = KeyCode::KeyS;
const TANK2_SHOOT_KEY: KeyCode = KeyCode::KeyQ;

const TANK_SIZE: Vec3 = Vec3::new(40.0,25.0,0.0);
const TANK_SPEED: f32 = 60.0;
const TANK_TURNING_SPEED: f32 = -2.0;

const BULLET_COLOR: Color = Color::GRAY;
const BULLET_RADIUS: f32 = 5.0;
// const BULLET_LIFETIME TODO
const BULLET_SPEED: f32 = 72.0;
const BULLET_WALL_COOLDOWN: f32 = 0.5;


#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Regenerate;

// TODO we will eventually need an id to enforce limited bullets
#[derive(Component)]
struct Tank {
    fwd_key: KeyCode,
    bwd_key: KeyCode,
    right_key: KeyCode,
    left_key: KeyCode,
    shoot_key: KeyCode,
}

#[derive(Bundle)]
struct TankBundle {
    tank: Tank,
    sprite: SpriteBundle,
}

// TODO bullet has limited lifetime
#[derive(Component)]
struct Bullet;

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    // sprite: MaterialMesh2dBundle<Handle<Color>>,
    sprite: MaterialMesh2dBundle<ColorMaterial>,
    velocity: Velocity,
    regenerate: Regenerate,
}

#[derive(Component)]
struct Wall {
    is_vertical: bool,
}

#[derive(Bundle)]
struct WallBundle {
    wall: Wall,
    sprite: SpriteBundle,
    regenerate: Regenerate,
}

#[derive(Event)]
struct FreshRound;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window : Some(Window {
            resolution: WINDOW_SIZE.into(),
            title: "Tank Trouble!".to_string(),
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(ClearColor(BACKGROUND_COLOR));

    #[cfg(feature = "debug")]
    // Debug hierarchy inspector
    app.add_plugins(WorldInspectorPlugin::new());

    app
    .add_event::<FreshRound>()
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Startup, setup)
    // TODO this should not go here we should probably just ping the exception in
    //  startup
    .add_systems(Update, (clear_prev_round,create_fresh_round).chain())
    .add_systems(Update, handle_keypresses)
    .add_systems(Update, (bullet_wall_collision_handler, apply_velocity).chain())
    .run()
}


fn setup(
    mut commands: Commands,
    mut start_game_event_writer: EventWriter<FreshRound>,
) {
    commands.spawn(Camera2dBundle::default());

    // TODO maybe modularize this
    commands.spawn(
        TankBundle {
            tank: Tank {
                fwd_key: TANK1_FWD_KEY,
                bwd_key: TANK1_BWD_KEY,
                right_key: TANK1_RIGHT_KEY,
                left_key: TANK1_LEFT_KEY,
                shoot_key: TANK1_SHOOT_KEY,
            },
            sprite: SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(0.0,0.0,0.0),
                    scale: TANK_SIZE,
                    ..default()
                },
                sprite: Sprite {
                    color: TANK1_COLOR,
                    ..default()
                },
                ..default()
            },
        }
    );

    commands.spawn(
        TankBundle {
            tank: Tank {
                fwd_key: TANK2_FWD_KEY,
                bwd_key: TANK2_BWD_KEY,
                right_key: TANK2_RIGHT_KEY,
                left_key: TANK2_LEFT_KEY,
                shoot_key: TANK2_SHOOT_KEY,
            },
            sprite: SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(0.0,0.0,0.0),
                    scale: TANK_SIZE,
                    ..default()
                },
                sprite: Sprite {
                    color: TANK2_COLOR,
                    ..default()
                },
                ..default()
            },
        }
    );

    start_game_event_writer.send(FreshRound);

}

fn clear_prev_round(
    mut commands: Commands,
    mut_query: Query<Entity, With<Regenerate>>,
    fresh_round_event: EventReader<FreshRound>,
) {
    if fresh_round_event.is_empty() {
        return;
    }

    for mut entity in mut_query.iter() {
        commands.entity(entity).despawn()
    }

}

fn generate_tank_starts(
    wall_mat: &Vec<Vec<(bool,bool)>>,
    rng: &mut ThreadRng,
) -> Vec<(usize,usize)> {
    // Find the biggest open section of the maze
    let mut visited:Vec<Vec<bool>> = Vec::new();
    for _ in 0..wall_mat.len() {
        visited.push(vec![false; wall_mat[0].len()]);
    }
    let mut clump_num_map: HashMap<(usize,usize),usize> = HashMap::new();

    let mut queue: VecDeque<(usize,usize)> = VecDeque::new();

    let mut curr_cluster = 0;
    let mut best_cluster = 0;
    let mut best_cluster_count = 0;

    for i in 0..wall_mat.len() {
        for j in 0..wall_mat[0].len() {
            if visited[i][j] {
                continue;
            }
            queue.push_back((i,j));
            visited[i][j] = true;

            let mut curr_cluster_count = 0;

            // Perform local bfs
            while !queue.is_empty() {
                let (curr_i, curr_j) = queue.pop_front().unwrap();
                clump_num_map.insert((curr_i,curr_j), curr_cluster);
                curr_cluster_count += 1;
                
                if curr_i > 0 && !wall_mat[curr_i-1][curr_j].1 && !visited[curr_i-1][curr_j] {
                    queue.push_back((curr_i-1,curr_j));
                    visited[curr_i-1][curr_j] = true;
                }
                if curr_i < wall_mat.len() - 1 && !wall_mat[curr_i][curr_j].1 && !visited[curr_i+1][curr_j] {
                    queue.push_back((curr_i+1,curr_j));
                    visited[curr_i+1][curr_j] = true;
                }
                if curr_j > 0 && !wall_mat[curr_i][curr_j-1].0 && !visited[curr_i][curr_j-1] {
                    queue.push_back((curr_i,curr_j-1));
                    visited[curr_i][curr_j-1] = true;
                }
                if curr_j < wall_mat[0].len() - 1 && !wall_mat[curr_i][curr_j].0 && !visited[curr_i][curr_j+1] {
                    queue.push_back((curr_i,curr_j+1));
                    visited[curr_i][curr_j+1] = true;
                }
            }

            if curr_cluster_count > best_cluster_count {
                best_cluster = curr_cluster;
                best_cluster_count = curr_cluster_count;
            }

            curr_cluster += 1;
        }
    }

    // Pick 2 random squares to place the tanks
    // Lolz they could end up on top of each other
    let mut starting_idxs: Vec<usize> = Vec::new();
    for _ in 0..TANK_COUNT {
        starting_idxs.push(rng.gen_range(0..best_cluster_count));
    }
    
    let mut starting_squares: Vec<(usize,usize)> = vec![(0,0); starting_idxs.len()];
    let mut curr_square = 0;
    for (key, val) in clump_num_map {
        if val == best_cluster {
            for clump_idx in 0..starting_idxs.len() {
                if starting_idxs[clump_idx] == curr_square {
                    starting_squares[clump_idx] = key;
                }
            }
            curr_square += 1;
        }
    }
    return starting_squares;
}


// TODO for debug purposes only
fn print_wall_mat_plus(
    wall_mat: &Vec<Vec<(bool,bool)>>,
) {
    for _ in 0..wall_mat.len() {
        print!("+-");
    }
    println!("+");
    for j in 0..(wall_mat[0].len()-1) {
        print!("| ");
        for i in 0..(wall_mat.len()-1) {
            if wall_mat[i][j].1 {
                print!("| ");
            } else {
                print!("  ");
            }
        }
        println!("|");
        for i in 0..wall_mat.len() {
            if wall_mat[i][j].0 {
                print!("+-");
            } else {
                print!("+ ");
            }
        }
        println!("+");
    }
    print!("| ");
        for i in 0..(wall_mat.len()-1) {
            if wall_mat[i][wall_mat[0].len()-1].1 {
                print!("| ");
            } else {
                print!("  ");
            }
        }
        println!("|");
    for _ in 0..wall_mat.len() {
        print!("+-");
    }
    println!("+");
}

fn print_wall_mat(
    wall_mat: &Vec<Vec<(bool,bool)>>,
) {
    for _ in 0..wall_mat.len() {
        print!(" -");
    }
    println!(" ");
    for j in 0..(wall_mat[0].len()-1) {
        print!("| ");
        for i in 0..(wall_mat.len()-1) {
            if wall_mat[i][j].1 {
                print!("| ");
            } else {
                print!("  ");
            }
        }
        println!("|");
        for i in 0..wall_mat.len() {
            if wall_mat[i][j].0 {
                print!(" -");
            } else {
                print!("  ");
            }
        }
        println!("+");
    }
    print!("| ");
        for i in 0..(wall_mat.len()-1) {
            if wall_mat[i][wall_mat[0].len()-1].1 {
                print!("| ");
            } else {
                print!("  ");
            }
        }
        println!("|");
    for _ in 0..wall_mat.len() {
        print!(" -");
    }
    println!(" ");
}

// NOTE: Coordinate system:
//  increasing "x" corresponds to increasing rightwards on the screen
//  increasing "Y" corresponds with downwards on the screen
fn generate_random_gameboard(
    (arena_x, arena_y): (usize, usize),
    rng: &mut ThreadRng,
) -> (Vec<(usize,usize)>, Box<impl FnMut() -> bool>, Box<impl FnMut() -> bool>) {
    

    // Generate representation of board -> 
    //  -> matrix of tuples -> (wall below, wall to right)

    let mut wall_mat: Vec<Vec<(bool,bool)>> = Vec::new();
    for _ in 0..arena_x {
        let mut wall_vec_temp: Vec<(bool, bool)> = Vec::new();
        for _ in 0..arena_y {
            // TODO Question, how do you get rng.gen type of polymorphism?
            // TODO question, does this take in an iterator??????
            wall_vec_temp.push(
                (rng.gen_range(0.0..1.0) < WALL_THRESHOLD,
                rng.gen_range(0.0..1.0) < WALL_THRESHOLD));
        }
        wall_mat.push(wall_vec_temp);
    }

    print_wall_mat(&wall_mat);

    let tank_starts = generate_tank_starts(&wall_mat, rng);

    // Generate easy values for the wall placers to iterate through
    // TODO Lol the closures just panic/unsafe stuff if they go out of bounds
    let wall_mat_rc_horz = Rc::new(wall_mat);
    let wall_mat_rc_vert = Rc::clone(&wall_mat_rc_horz);

    let mut idx_horz_i = 0;
    let mut idx_horz_j = 0;
    let horz_fun = 
        Box::new(move || -> bool {
            let ret = 
                if idx_horz_j == 0 || idx_horz_j == arena_y {
                    true
                } else {
                    wall_mat_rc_horz[idx_horz_i][idx_horz_j-1].0
                };
            idx_horz_j += 1; 
            if idx_horz_j > arena_y {
                idx_horz_j = 0; 
                idx_horz_i += 1;
            };
            ret
            });
    let mut idx_vert_i = 0;
    let mut idx_vert_j = 0;
    let vert_fun =
        Box::new(move || -> bool {
            let ret =
                if idx_vert_i == 0 || idx_vert_i == arena_x {
                    true
                } else {
                    wall_mat_rc_vert[idx_vert_i - 1][idx_vert_j].1
                };

            idx_vert_i += 1;
            if idx_vert_i > arena_x {
                idx_vert_i = 0;
                idx_vert_j += 1;
            };
            ret
        });

    return (tank_starts,horz_fun,vert_fun);

}

// TODO have this run on an event that gets triggered on startup and on new game
// TODO make this depend on another step that clears all the things
fn create_fresh_round(
    mut commands: Commands,
    // Need tanks so we can move them first round
    // TODO?
    // mut tanks_query: Query<(&mut Transform, &Tank)>,
    mut tanks_query: Query<&mut Transform, With<Tank>>,
    fresh_round_event: EventReader<FreshRound>,
) {
    if fresh_round_event.is_empty() {
        return;
    }

    let mut rng = rand::thread_rng();
    
    // TODO actually make walls once we see that this runs
    let (tank_positions, 
        mut horz_fun, 
        mut vert_fun) 
            = generate_random_gameboard(ARENA_DIM, &mut rng);

    // TODO shitty code but may refactor once make screen resizeable
    let x_step = (WINDOW_SIZE.0 / (ARENA_DIM.0 as f32));
    let y_step = (WINDOW_SIZE.1 / (ARENA_DIM.1 as f32));
    // TODO lol assumes we have an even numer of boxes in a dim
    for i in 0..ARENA_DIM.0 {
        let x_pos = ((i as f32) * x_step) - ((WINDOW_SIZE.0 - x_step) / 2.0);
        for j in 0..=ARENA_DIM.1 {
            let y_pos = -((j as f32) * y_step) + (WINDOW_SIZE.1 / 2.0);
            if horz_fun() {
            // if true {
                commands.spawn(
                    WallBundle {
                        wall: Wall {
                            is_vertical: false,
                        },
                        sprite: SpriteBundle {
                            transform: Transform {
                                translation: Vec3::new(x_pos,y_pos,0.0),
                                rotation: Quat::from_rotation_z(0.0),
                                scale: WALL_SIZE,
                            },
                            sprite: Sprite {
                                color: WALL_COLOR,
                                ..default()
                            },
                            ..default()
                        },
                        regenerate: Regenerate,
                    }
                );
            }
        }
    }

    for i in 0..ARENA_DIM.1 {
        let y_pos = -((i as f32) * y_step) + ((WINDOW_SIZE.1 - y_step) / 2.0);

        for j in 0..=ARENA_DIM.0 {
            let x_pos = ((j as f32) * x_step) - (WINDOW_SIZE.0 / 2.0);
        
            if vert_fun() {
                commands.spawn(
                    WallBundle {
                        wall: Wall {
                            is_vertical: true,
                        },
                        sprite: SpriteBundle {
                            transform: Transform {
                                translation: Vec3::new(x_pos,y_pos,0.0),
                                rotation: Quat::from_rotation_z(PI / 2.0),
                                scale: WALL_SIZE,
                            },
                            sprite: Sprite {
                                color: WALL_COLOR,
                                ..default()
                            },
                            ..default()
                        },
                        regenerate: Regenerate,
                    }
                );
            }
            
        }
    }

    let mut tank_positions_idx = 0;
    // TODO
    // for (mut transform,_) in &mut tanks_query {
    for mut transform in &mut tanks_query {
        // Calculate screen coordinate based on square coordinate
        let x_coord = tank_positions[tank_positions_idx].0;
        let y_coord = tank_positions[tank_positions_idx].1;
        // TODO copied code from prev step
        let x_pos = ((x_coord as f32) * x_step) - ((WINDOW_SIZE.0 - x_step) / 2.0);
        let y_pos = -((y_coord as f32) * y_step) + ((WINDOW_SIZE.1 - y_step) / 2.0);

        println!("!!!!!!!!!!!!!!!!!!!new tank pos {}: ({},{})",tank_positions_idx,x_pos,y_pos);
        transform.translation = Vec3::new(x_pos,y_pos,0.0);
        transform.rotation = Quat::from_rotation_z(rng.gen_range(-PI..PI));
        tank_positions_idx += 1;
    }

}

// TODO question, what is the difference beween having the mut on the right and
//  the mut on the left?


// TODO hmmm probably bad use of ecs, probably want a subclass of things that 
//  can shoot/be shot. Should not ideally pass params like this
fn shoot_bullet(
    commands: &mut Commands,
    angle: &f32,
    tank_pos: &Vec3,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {

    let circ_mesh = Mesh2dHandle(meshes.add(Circle { radius: BULLET_RADIUS }));
    let color_material = materials.add(BULLET_COLOR);

    // TODO could potentially be redone to use rotating point api
    let start_x = tank_pos.x + angle.cos() * ((TANK_SIZE.x / 2.0) + BULLET_RADIUS);
    let start_y = tank_pos.y + angle.sin() * ((TANK_SIZE.x / 2.0) + BULLET_RADIUS);

    // TODO turn this into a ball?
    commands.spawn(
        BulletBundle {
            bullet: Bullet,
            sprite: MaterialMesh2dBundle {
                mesh: circ_mesh,
                material: color_material,
                transform: Transform {
                    translation: Vec3::new(start_x, start_y, 0.0),
                    ..default()
                },
                ..default()
            },
            velocity: Velocity(Vec2::new(angle.cos() * BULLET_SPEED, angle.sin() * BULLET_SPEED)),
            regenerate: Regenerate,
        }
    );
}

fn move_tank(
    commands: &mut Commands,
    keys: &Res<ButtonInput<KeyCode>>,
    transform: &mut Transform,
    tank: &Tank,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    time: &Res<Time>,
) {

    // TODO once implemented collision, only allow to turn or move if movement
    //  won't cause collision with wall
    if keys.pressed(tank.right_key) && !keys.pressed(tank.left_key) {
        transform.rotate_z(TANK_TURNING_SPEED * time.delta_seconds());
    } else if keys.pressed(tank.left_key) && !keys.pressed(tank.right_key) {
        transform.rotate_z(-TANK_TURNING_SPEED * time.delta_seconds());
    }
    
    let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
    if keys.pressed(tank.fwd_key) && !keys.pressed(tank.bwd_key) {
        transform.translation.x += angle.cos() * TANK_SPEED * time.delta_seconds();
        transform.translation.y += angle.sin() * TANK_SPEED * time.delta_seconds();        
    } else if keys.pressed(tank.bwd_key) && !keys.pressed(tank.fwd_key) {
        transform.translation.x += angle.cos() * (-TANK_SPEED) * time.delta_seconds();
        transform.translation.y += angle.sin() * (-TANK_SPEED) * time.delta_seconds();  
    }

    if keys.just_pressed(tank.shoot_key) {
        shoot_bullet(commands, &angle, &transform.translation, meshes, materials);
    }

}

// TODO maybe add to own submodule-esque thing?
fn handle_keypresses(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut tanks_query: Query<(&mut Transform,  &Tank)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
) {
    for (mut transform, tank) in &mut tanks_query {
        move_tank(&mut commands, &keys, &mut transform, tank, &mut meshes, &mut materials, &time);
    }
}

fn apply_velocity(
    mut query: Query<(&mut Transform, &Velocity)>,
    time: Res<Time>,
) {
    for (mut transform, velocity) in &mut query {
        // TODO add time eventually
        transform.translation.x += velocity.x * time.delta_seconds();
        transform.translation.y += velocity.y * time.delta_seconds();
    }
}



// Collision functions

// TODO derive rectangle hitbox

// Circle with rectangle

// Vec2 Rotate(Vec2 point, float angle, Vec2 center_of_rotation)
// {
//     float sinus   = sin(angle)
//     float cosinus = cos(angle);
//     Vec2 temp;

//     point  = point - center_of_rotation;
//     temp.x = point.x * cosinus - point.y * sinus;
//     temp.y = point.x * sinus   + point.y * cosinus;
//     point  =  temp + center_of_rotation;

//     return point;
// }

fn rotate_point(point: Vec2, angle: f32, rot_center: Vec2) -> Vec2 {
    let temp = point - rot_center;

    let temp2 = Vec2::new(
        temp.x * angle.cos() - temp.y * angle.sin(),
    temp.x * angle.sin() + temp.y * angle.cos());
    
    return temp2 + rot_center;
}


fn circle_intersects_rect(
    rect_center: Vec2,
    rect_dims: Vec2,
    rect_angle: f32,
    circle_center: Vec2,
    circle_rad: f32,
) -> bool {
    let circle_center_rot = rotate_point(circle_center, rect_angle, rect_center);

    let circle_dist = Vec2::new(
        (circle_center_rot.x - rect_center.x).abs(),
        (circle_center_rot.y - rect_center.y).abs());

    if circle_dist.x > (rect_dims.x/2.0) + circle_rad {return false}

    if circle_dist.y > (rect_dims.y/2.0) + circle_rad {return false}

    if circle_dist.x <= (rect_dims.x/2.0) {return true}

    if circle_dist.y <= (rect_dims.y/2.0) {return true}

    let corner_distance_sq = (circle_dist.x - (rect_dims.x/2.0)).powf(2.0) + 
        (circle_dist.y - (rect_dims.y/2.0)).powf(2.0);
    
    return corner_distance_sq <= circle_rad.powf(2.0);
}


fn circle_intersects_wall_bounce(
    wall_center: Vec2,
    wall_width: f32,
    wall_vertical: bool,
    circle_center: Vec2,
    circle_rad: f32,
    circle_velo: Vec2,
) -> bool {

    if wall_vertical {
        if (circle_velo.x > 0.0 && (circle_center.x - wall_center.x < 0.0))
        || (circle_velo.x < 0.0 && (circle_center.x - wall_center.x > 0.0)) {

            return ((circle_center.y - wall_center.y).abs() < (wall_width/2.0)) 
            && ((circle_center.x - wall_center.x).abs() <= circle_rad)
        } else {
            return false;
        }
    } else {
        if (circle_velo.y > 0.0 && (circle_center.y - wall_center.y < 0.0))
        || (circle_velo.y < 0.0 && (circle_center.y - wall_center.y > 0.0)) {
        
            return ((circle_center.x - wall_center.x).abs() < (wall_width/2.0))
            && ((circle_center.y - wall_center.y).abs() <= circle_rad)
        } else {
            return false;
        }
    }
}



fn bullet_wall_collision_handler(
    walls_query: Query<(&Transform, &Wall)>,
    mut bullets_query: Query<(&Transform, &mut Velocity), With<Bullet>>,
) {
    for (bullet_trans, mut bullet_velo) in &mut bullets_query {
        for (wall_trans, wall) in &walls_query {
            
            if circle_intersects_wall_bounce(
                Vec2::new(wall_trans.translation.x, wall_trans.translation.y),
                wall_trans.scale.x,
                wall.is_vertical,
                Vec2::new(bullet_trans.translation.x, bullet_trans.translation.y), 
                BULLET_RADIUS,
                bullet_velo.0) {
                
                if wall.is_vertical {
                    bullet_velo.x = -bullet_velo.x;
                } else {
                    bullet_velo.y = -bullet_velo.y;
                }
            }
        }
    }
}


fn bullet_tank_collision_handler(

) {

}







/*
TODO when need to intersect with tank
let temp = circle_intersects_wall_side(
                Vec2::new(wall_trans.translation.x, wall_trans.translation.y),
                Vec2::new(wall_trans.scale.x, wall_trans.scale.y),
                wall_trans.rotation.to_euler(EulerRot::ZYX).0,
                Vec2::new(bullet_trans.translation.x, bullet_trans.translation.y), 
                BULLET_RADIUS);
*/



// TODO question: why this make rust sad?
// fn makeMeAMoney() -> Fn() -> i32 {
    
//     let mut eye: Vec<i32> = vec![1,2,3];
//     let mut idx: usize = 0;
//     return ||-> i32 { idx += 1; eye[idx - 1]};
// }

// fn main() {
//     let thang = makeMeAMoney();
//     println!("thang {}",thang());
// }