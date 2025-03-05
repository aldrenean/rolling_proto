use bevy::{
    math::bounding::{BoundingSphere, IntersectsVolume, RayCast3d},
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use rand::Rng;

pub mod camera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .init_gizmo_group::<DefaultGizmoConfigGroup>()
        .add_systems(
            Startup,
            (create_assets, (camera::spawn_camera, spawn_initial_targets)).chain(),
        )
        .add_systems(Update, (camera::camera_control, aim_check))
        .add_systems(FixedUpdate, camera::normalize_aim)
        .run();
}

#[derive(Component)]
struct MyBoundingSphere(BoundingSphere);

#[derive(Bundle)]
struct Shape {
    visibility: Visibility,
    transform: Transform,
    mesh: Mesh3d,
    material: MeshMaterial3d<StandardMaterial>,
}

#[derive(Bundle)]
struct Target {
    shape: Shape,
    bounding: MyBoundingSphere,
    state: TargetState,
}

#[derive(Component, PartialEq, Eq)]
enum TargetState {
    Active,
    Next,
    Ghost,
}

#[derive(Resource)]
pub struct MyAssets {
    debug_material: Handle<StandardMaterial>,
    debug_target_mesh: Handle<Mesh>,
    arrow: Handle<StandardMaterial>,
    arrow_faded: Handle<StandardMaterial>,
}

impl Default for Shape {
    fn default() -> Self {
        Shape {
            visibility: Visibility::Visible,
            transform: Transform::default(),
            mesh: Mesh3d::default(),
            material: MeshMaterial3d::default(),
        }
    }
}

const MAX_RADIUS: f32 = 20.0;
const TARGET_RADIUS: f32 = 1.;
const TARGET_DISTANCE: f32 = 8.;
const DEADZONE_RADIUS_SQUARED: f32 = 4.;
const DEADZONE_ADJ_THETA: f32 = -0.02;

fn create_assets(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    external_assets: Res<AssetServer>,
) {
    let texture_data = [0; 64 * 4];
    let texture = Image::new_fill(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(texture)),
        ..default()
    });

    let arrow_texture: Handle<Image> = external_assets.load("arrow.png");
    let arrow_material = materials.add(arrow_texture);

    let arrow_faded_texture: Handle<Image> = external_assets.load("arrow_faded.png");
    let arrow_faded_material = materials.add(arrow_faded_texture);

    let shape = Sphere::new(TARGET_RADIUS);
    let debug_target_mesh = meshes.add(shape);

    commands.insert_resource(MyAssets {
        debug_material,
        debug_target_mesh,
        arrow: arrow_material,
        arrow_faded: arrow_faded_material,
    });
}

fn draw_gizmos(
    mut gizmos: Gizmos,
    q_camera: Query<&Transform, With<camera::CameraSettings>>,
    mut q_target: Query<(&Transform, &TargetState), With<MyBoundingSphere>>,
) {
    if let Ok(cam_transform) = q_camera.get_single() {
        for (&target_pos, state) in &mut q_target {
            if *state == TargetState::Active {
                let arrow_base = cam_transform
                    .forward()
                    .slerp(cam_transform.local_x(), 0.05)
                    .slerp(-cam_transform.local_y(), 0.03)
                    .as_vec3();
                gizmos.arrow(arrow_base, target_pos.translation, Color::BLACK);
            }
            if *state == TargetState::Next {
                // aim_point = Some(target_pos.translation);
            }
            gizmos.axes(target_pos, 2.0);
        }

        // gizmos.axes(Transform::from_xyz(0., 0., 0.), 2.0);
    }
}

// Next step: there should always be two targets, one active and one next. Next target should be faded and not hittable.
// Need pointer or other hint leading to active target. This code as-is can be spawning the next target, but we'll need to spawn
// two at game-loop start.
// Actually do we need three targets? Active, Next, & Ghost? This way we can orient Next before it becomes active.
fn spawn_initial_targets(mut commands: Commands, my_assets: Res<MyAssets>) {
    let ghost_pos = spawn_target(&mut commands, &my_assets, TargetState::Ghost, None, None);
    let next_pos = spawn_target(
        &mut commands,
        &my_assets,
        TargetState::Next,
        Some(ghost_pos),
        Some(ghost_pos),
    );
    let _active_pos = spawn_target(
        &mut commands,
        &my_assets,
        TargetState::Active,
        Some(next_pos),
        Some(next_pos),
    );
}

fn random_normalized_vec3() -> Vec3 {
    let mut gen = rand::thread_rng();
    Vec3::new(
        gen.gen_range(-1.0..1.0),
        gen.gen_range(-1.0..1.0),
        gen.gen_range(-1.0..1.0),
    )
    .normalize()
}

fn aim_check(
    commands: Commands,
    q_camera: Query<&Transform, (With<camera::CameraSettings>, Without<TargetState>)>,
    mut q_target: Query<(
        Entity,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut TargetState,
        &mut Visibility,
        &MyBoundingSphere,
    )>,
    my_assets: Res<MyAssets>,
) {
    if let Ok(transform) = q_camera.get_single() {
        let aim = transform.forward();

        // Get a ray coming out the barrel of the camera
        let ray = RayCast3d::new(transform.translation, aim, MAX_RADIUS);
        let mut hit = false;
        let mut active_id: Option<Entity> = None;
        let mut next_state: Option<Mut<TargetState>> = None;
        let mut next_mat: Option<Mut<MeshMaterial3d<StandardMaterial>>> = None;
        let mut ghost_state: Option<(Mut<Transform>, Mut<TargetState>, Mut<Visibility>)> = None;

        for (id, transform, material, state, visibility, bounding) in &mut q_target {
            match *state {
                TargetState::Active => {
                    active_id = Some(id);
                    // active_state = Some(*state);
                    if ray.intersects(&bounding.0) {
                        hit = true;
                    }
                }
                TargetState::Next => {
                    // state = TargetState::Active;
                    // next_id = Some(id);
                    next_state = Some(state);
                    next_mat = Some(material);
                    // next_pos = Some(transform.translation);
                }
                TargetState::Ghost => {
                    // ghost_id = Some(id);
                    ghost_state = Some((transform, state, visibility));
                }
            }
        }
        if hit & (active_id != None) {
            if let Some(mut old_next_state) = next_state {
                // Make old next new active & un-fade the texture
                *old_next_state = TargetState::Active;
                if let Some(mut old_next_mat) = next_mat {
                    *old_next_mat = MeshMaterial3d(my_assets.arrow.clone());
                }
                if let Some((mut old_ghost_transform, mut old_ghost_state, mut old_ghost_vis)) =
                    ghost_state
                {
                    // Cycle old ghost to be new next
                    *old_ghost_state = TargetState::Next;
                    *old_ghost_vis = Visibility::Visible;

                    // Make new ghost target
                    let new_ghost_pos = target_hit(
                        commands,
                        my_assets,
                        active_id.unwrap(),
                        Some(old_ghost_transform.translation),
                    );

                    // Point new next at new ghost
                    orient_target(&mut old_ghost_transform, new_ghost_pos);
                } else {
                    panic!("Ghost target missing!");
                }
            } else {
                panic!("Next target missing!");
            }
        }
    }
}

fn target_hit(
    mut commands: Commands,
    my_assets: Res<MyAssets>,
    hit_target_id: Entity,
    deadzone: Option<Vec3>,
) -> Vec3 {
    // Trigger any visual or audio effects on hit, play fade animation
    // Spawn note-after-next
    commands.entity(hit_target_id).despawn_recursive();
    spawn_target(
        &mut commands,
        &my_assets,
        TargetState::Ghost,
        None,
        deadzone,
    )
}

fn spawn_target(
    commands: &mut Commands,
    my_assets: &Res<MyAssets>,
    state: TargetState,
    aim_point: Option<Vec3>,
    deadzone: Option<Vec3>,
) -> Vec3 {
    let mut target_center = random_normalized_vec3() * TARGET_DISTANCE;

    if let Some(deadzone) = deadzone {
        let btwn = target_center - deadzone;
        let distance_sq = btwn.length_squared();
        println!("dist is {:?} before adjustment", distance_sq);
        if distance_sq < DEADZONE_RADIUS_SQUARED {
            let rot_axis = btwn.cross(target_center);
            let rot = Quat::from_axis_angle(rot_axis, DEADZONE_ADJ_THETA);
            target_center = rot.mul_vec3(target_center);
            println!("dist is {:?} after adjustment", distance_sq);
        }
    }

    let model = my_assets.debug_target_mesh.clone();
    let bounding = MyBoundingSphere(BoundingSphere::new(target_center, TARGET_RADIUS));

    let mat = match state {
        TargetState::Active => my_assets.arrow.clone(),
        TargetState::Next | TargetState::Ghost => my_assets.arrow_faded.clone(),
    };

    let mut target = Target {
        shape: Shape {
            mesh: Mesh3d(model),
            material: MeshMaterial3d(mat),
            transform: Transform {
                translation: target_center,
                ..Default::default()
            },
            visibility: match state {
                TargetState::Active | TargetState::Next => Visibility::Visible,
                TargetState::Ghost => Visibility::Hidden,
            },
        },
        bounding,
        state,
    };

    let dir_to_center = -target_center.normalize();
    let face_to_center = Quat::from_rotation_arc(Vec3::X, dir_to_center);
    target.shape.transform.rotation = face_to_center;

    if let Some(aim_point) = aim_point {
        orient_target(&mut target.shape.transform, aim_point);
    }

    commands.spawn(target);
    target_center
}

fn orient_target(transform: &mut Transform, aim_point: Vec3) {
    let dir_to_center = -transform.translation.normalize();
    transform.align(Dir3::X, dir_to_center, Dir3::Y, aim_point);
}

// fn next_note() {
// Code to semi-randomly determine the next musical note in the progression,
// and use that to determine where the next target will spawn
// let base = chord.base_note;
// }

// fn background_music() {
// Might not want to be a function, but this should handle the background music,
// which should be a sensible chord progression that gates the possible values for
// next_note. Also maybe a rising shepherd tone?
// }
