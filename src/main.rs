

// commands despawn

use bevy::
{ 
    input::{keyboard::{Key, KeyboardInput}, ButtonState}, 
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle}
};

#[cfg(feature = "debug")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;


/* TODO for now hardcode all the sizes for simplicity. In the future, can 
  elaborate more regarding resizeability and scale. The minesweeper example
  does a good job of this
*/

const BACKGROUND_COLOR: Color = Color::WHITE;

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

const WALL_COLOR: Color = Color::BLACK;
// const WALL_SIZE TODO -> fixed for now I guess

const BULLET_COLOR: Color = Color::GRAY;
const BULLET_RADIUS: f32 = 5.0;
// const BULLET_LIFETIME TODO
const BULLET_SPEED: f32 = 72.0;

// const ARENA_DIM


#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

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

#[derive(Component)]
struct Bullet {

}

#[derive(Bundle)]
struct BulletBundle {
    bullet: Bullet,
    // sprite: MaterialMesh2dBundle<Handle<Color>>,
    sprite: MaterialMesh2dBundle<ColorMaterial>,
    velocity: Velocity,
}


fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window : Some(Window {
            resolution: (800.0,700.0).into(),
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
    .add_systems(Update, bevy::window::close_on_esc)
    .add_systems(Startup, setup)
    .add_systems(Update, handle_keypresses)
    .add_systems(Update, apply_velocity)
    .run()
}


fn setup(
    mut commands: Commands
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
    time: &Res<Time>,
) {

    let circ_mesh = Mesh2dHandle(meshes.add(Circle { radius: BULLET_RADIUS }));
    let color_material = materials.add(BULLET_COLOR);

    let start_x = tank_pos.x + angle.cos() * ((TANK_SIZE.x / 2.0) + BULLET_RADIUS);
    let start_y = tank_pos.y + angle.sin() * ((TANK_SIZE.x / 2.0) + BULLET_RADIUS);

    // TODO turn this into a ball?
    commands.spawn(
        BulletBundle {
            bullet: Bullet {},
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
        shoot_bullet(commands, &angle, &transform.translation, meshes, materials, time);
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