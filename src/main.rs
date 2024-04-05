/*
!!!!!!!!WARNING!!!!!!!!

DOG SHIT QUALITY CODE INCOMING

Before doing this project, I knew nothing about both rust and game engines, 
and once I actually did figure out a bit about both, I was in too much of a 
rush to properly structure the project. As such, we now have a 1k line file 
full of spaghetti code and a bunch of TODO statements...

Have fun!

*/






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

const GAME_RESTART_WAIT: Duration = Duration::from_secs(3);

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

const TANK_SIZE: Vec3 = Vec3::new(30.0,22.0,0.0);
const TANK_SPEED: f32 = 110.0;
const TANK_TURNING_SPEED: f32 = -4.0;
const DEAD_TANK_COLOR: Color = Color::BLUE;
const TANK_BULLET_COUNT: usize = 5;

const BULLET_COLOR: Color = Color::GRAY;
const BULLET_RADIUS: f32 = 5.0;
const BULLET_EXPIRATION: Duration = Duration::from_secs(17);
const BULLET_SPEED: f32 = 120.0;


#[derive(Resource)]
struct GlobalRestart {
    timer: Timer,
    restart: bool,
}


#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Regenerate;

#[derive(Component)]
struct Expiration {
    timer: Timer,
}

// TODO we will eventually need an id to enforce limited bullets
#[derive(Component)]
struct Tank {
    fwd_key: KeyCode,
    bwd_key: KeyCode,
    right_key: KeyCode,
    left_key: KeyCode,
    shoot_key: KeyCode,
    tank_id: usize,
    bullets_remaining: usize,
}

#[derive(Bundle)]
struct TankBundle {
    tank: Tank,
    sprite: SpriteBundle,
}

// TODO bullet has limited lifetime
#[derive(Component)]
struct Bullet {
    tank_id: usize,
}

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    // sprite: MaterialMesh2dBundle<Handle<Color>>,
    sprite: MaterialMesh2dBundle<ColorMaterial>,
    velocity: Velocity,
    regenerate: Regenerate,
    expiration: Expiration,
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
    .insert_resource(GlobalRestart {restart:false, timer: Timer::new(Duration::from_secs(0), TimerMode::Once)})
    .add_event::<FreshRound>()
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Startup, setup)
    // TODO this should not go here we should probably just ping the exception in
    //  startup
    .add_systems(Update, (handle_restarting_game, clear_prev_round,create_fresh_round).chain())
    .add_systems(Update, handle_keypresses)
    .add_systems(Update, game_end_condition_handler)
    .add_systems(Update, (bullet_wall_collision_handler, apply_velocity).chain())
    .add_systems(Update, handle_expiring_bullets)
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
                tank_id: 1,
                bullets_remaining: TANK_BULLET_COUNT,
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
                tank_id: 2,
                bullets_remaining: TANK_BULLET_COUNT,
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

    for entity in mut_query.iter() {
        commands.entity(entity).despawn()
    }

}

fn handle_restarting_game(
    mut fresh_round_event_writer: EventWriter<FreshRound>,
    global_restart: Res<GlobalRestart>,
) {
    if global_restart.restart && global_restart.timer.finished() {
        fresh_round_event_writer.send(FreshRound);
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
    mut tanks_query: Query<(&mut Transform, &mut Sprite, &mut Tank)>,
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
    for (mut transform, mut sprite, mut tank) in &mut tanks_query {
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

        tank.bullets_remaining = TANK_BULLET_COUNT;

        // TODO shitty hard casing, idk if can change though...
        if tank.tank_id == 1 {
            sprite.color = TANK1_COLOR;
        } else if tank.tank_id == 2 {
            sprite.color = TANK2_COLOR;
        } else {
            panic!();
        }
    }

}


fn handle_expiring_bullets(
    mut commands: Commands,
    mut expiration_query: Query<(Entity, &mut Expiration, &Bullet)>,
    mut tank_query: Query<&mut Tank>,
    time: Res<Time>,
) {
    for (entity, mut expiration, bullet) in &mut expiration_query {
        expiration.timer.tick(time.delta());

        if expiration.timer.finished() {
            for mut tank in &mut tank_query {
                if tank.tank_id == bullet.tank_id {
                    tank.bullets_remaining += 1;
                }
            }

            commands.entity(entity).despawn();
        }
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
    tank: &mut Tank,
) {
    if tank.bullets_remaining == 0 {
        return
    }

    let circ_mesh = Mesh2dHandle(meshes.add(Circle { radius: BULLET_RADIUS }));
    let color_material = materials.add(BULLET_COLOR);

    // TODO could potentially be redone to use rotating point api
    let start_x = tank_pos.x + angle.cos() * ((TANK_SIZE.x / 2.0) + 2.0 * BULLET_RADIUS);
    let start_y = tank_pos.y + angle.sin() * ((TANK_SIZE.x / 2.0) + 2.0 * BULLET_RADIUS);

    tank.bullets_remaining -= 1;
    commands.spawn(
        BulletBundle {
            bullet: Bullet {
                tank_id: tank.tank_id,
            },
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
            expiration: Expiration {
                timer: Timer::new(BULLET_EXPIRATION, TimerMode::Once),
            },
        }
    );
}

fn move_tank(
    commands: &mut Commands,
    keys: &Res<ButtonInput<KeyCode>>,
    transform: &mut Transform,
    tank: &mut Tank,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    time: &Res<Time>,
    walls_query: &Query<(&Transform, &Wall), Without<Tank>>,
) {
    let mut transform_pending = transform.clone();

    // TODO once implemented collision, only allow to turn or move if movement
    //  won't cause collision with wall
    if keys.pressed(tank.right_key) && !keys.pressed(tank.left_key) {
        transform_pending.rotate_z(TANK_TURNING_SPEED * time.delta_seconds());
    } else if keys.pressed(tank.left_key) && !keys.pressed(tank.right_key) {
        transform_pending.rotate_z(-TANK_TURNING_SPEED * time.delta_seconds());
    }
    
    let angle_pending = transform_pending.rotation.to_euler(EulerRot::ZYX).0;
    if keys.pressed(tank.fwd_key) && !keys.pressed(tank.bwd_key) {
        transform_pending.translation.x += angle_pending.cos() * TANK_SPEED * time.delta_seconds();
        transform_pending.translation.y += angle_pending.sin() * TANK_SPEED * time.delta_seconds();        
    } else if keys.pressed(tank.bwd_key) && !keys.pressed(tank.fwd_key) {
        transform_pending.translation.x += angle_pending.cos() * (-TANK_SPEED) * time.delta_seconds();
        transform_pending.translation.y += angle_pending.sin() * (-TANK_SPEED) * time.delta_seconds();  
    }

    // Check to make sure pending version does not intersect with walls
    let rect_8_f32 = rect_to_8_f23(transform_pending);
    let mut intersects = false;
    for (wall_trans, wall) in walls_query {
        let wall_line = if wall.is_vertical {
            (wall_trans.translation.x,
                wall_trans.translation.y - (wall_trans.scale.x/2.0),
                wall_trans.translation.x,
                wall_trans.translation.y + (wall_trans.scale.x/2.0))
        } else {
            (wall_trans.translation.x - (wall_trans.scale.x/2.0),
                wall_trans.translation.y,
                wall_trans.translation.x + (wall_trans.scale.x/2.0),
                wall_trans.translation.y)
        };
        if rectangle_intersects_line(rect_8_f32, wall_line) {
            intersects = true;
            break;
        }
    }
    // If does not intersect with walls, set as real value
    if !intersects {
        transform.translation = transform_pending.translation;
        transform.rotation = transform_pending.rotation;
    }

    let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
    if keys.just_pressed(tank.shoot_key) {
        shoot_bullet(commands, &angle, &transform.translation, meshes, materials, tank);
    }

}

// TODO maybe add to own submodule-esque thing?
fn handle_keypresses(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut tanks_query: Query<(&mut Transform, &mut Tank)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    walls_query: Query<(&Transform, &Wall), Without<Tank>>,
) {
    for (mut transform, mut tank) in &mut tanks_query {
        move_tank(&mut commands, &keys, &mut transform, &mut tank, &mut meshes, &mut materials, &time, &walls_query);
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

fn rotate_point(point: &Vec2, angle: &f32, rot_center: &Vec2) -> Vec2 {
    let temp = *point - *rot_center;

    let temp2 = Vec2::new(
        temp.x * angle.cos() - temp.y * angle.sin(),
    temp.x * angle.sin() + temp.y * angle.cos());
    
    return temp2 + *rot_center;
}

fn circle_intersects_rect(
    rect_center: Vec2,
    rect_dims: Vec2,
    rect_angle: f32,
    circle_center: Vec2,
    circle_rad: f32,
) -> bool {
    let circle_center_rot = rotate_point(&circle_center, &rect_angle, &rect_center);

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

fn line_intersects_line(
    (x1, y1, x2, y2): (f32,f32,f32,f32),
    (x3, y3, x4, y4): (f32,f32,f32,f32),
) -> bool {
    let u_a = ((x4-x3)*(y1-y3) - (y4-y3)*(x1-x3)) / ((y4-y3)*(x2-x1) - (x4-x3)*(y2-y1));
    let u_b = ((x2-x1)*(y1-y3) - (y2-y1)*(x1-x3)) / ((y4-y3)*(x2-x1) - (x4-x3)*(y2-y1));

    u_a >= 0.0 && u_a <= 1.0 && u_b >= 0.0 && u_b <= 1.0
}

fn rectangle_intersects_line(
    rect: (f32,f32,f32,f32,f32,f32,f32,f32),
    line: (f32,f32,f32,f32),
) -> bool {
    let (x1,y1,x2,y2,x3,y3,x4,y4) = rect;

    let left = line_intersects_line(line, (x1,y1,x2,y2));
    let right = line_intersects_line(line, (x2,y2,x3,y3));
    let top = line_intersects_line(line, (x3,y3,x4,y4));
    let bottom = line_intersects_line(line, (x4,y4,x1,y1));

    left || right || top || bottom
}

fn rect_to_8_f23(transform: Transform) -> (f32,f32,f32,f32,f32,f32,f32,f32) {
    let horz_offset = transform.scale.x / 2.0;
    let vert_offset = transform.scale.y / 2.0;
    let angle = transform.rotation.to_euler(EulerRot::ZYX).0;
    let center_vec2 = Vec2::new(transform.translation.x, transform.translation.y);

    let p1 = Vec2::new(center_vec2.x - horz_offset, center_vec2.y - vert_offset);
    let p2 = Vec2::new(center_vec2.x + horz_offset, center_vec2.y - vert_offset);
    let p3 = Vec2::new(center_vec2.x - horz_offset, center_vec2.y + vert_offset);
    let p4 = Vec2::new(center_vec2.x + horz_offset, center_vec2.y + vert_offset);

    let Vec2{x:x1,y:y1} = rotate_point(&p1, &angle, &center_vec2);
    let Vec2{x:x2,y:y2} = rotate_point(&p2, &angle, &center_vec2);
    let Vec2{x:x3,y:y3} = rotate_point(&p3, &angle, &center_vec2);
    let Vec2{x:x4,y:y4} = rotate_point(&p4, &angle, &center_vec2);

    (x1,y1,x2,y2,x3,y3,x4,y4)
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



fn game_end_condition_handler(
    bullets_query: Query<&Transform, With<Bullet>>,
    mut tanks_query: Query<(&Transform, &mut Sprite), With<Tank>>,
    fresh_round_event_reader: EventReader<FreshRound>,
    mut global_restart: ResMut<GlobalRestart>,
    timer: Res<Time>,
) {
    global_restart.timer.tick(timer.delta());

    if !fresh_round_event_reader.is_empty() {
        global_restart.restart = false;
    }

    if global_restart.restart {
        return
    }

    for (tank_transform, mut tank_sprite) in &mut tanks_query {
        for bullets_transform in &bullets_query {

            if circle_intersects_rect(
                Vec2::new(tank_transform.translation.x, tank_transform.translation.y), 
                Vec2::new(tank_transform.scale.x, tank_transform.scale.y), 
                tank_transform.rotation.to_euler(EulerRot::ZXY).0, 
                Vec2::new(bullets_transform.translation.x, bullets_transform.translation.y), 
                BULLET_RADIUS) {

                tank_sprite.color = DEAD_TANK_COLOR;

                global_restart.restart = true;
                global_restart.timer = Timer::new(GAME_RESTART_WAIT, TimerMode::Once);
            }
        }
    }
    // all this should do is hide the dead tank, wait 3 secs, and send restart
    //  signal
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