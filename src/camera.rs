use bevy::prelude::*;
use std::f32::consts::PI;

#[derive(Bundle, Default)]
struct MyCameraBundle {
    camera: Camera3d,
    transform: Transform,
    state: CameraAimState,
    settings: CameraSettings,
}

#[derive(Component)]
pub struct CameraSettings {
    // fov: u16,
    pos: Vec3,
    pitch_rate: f32,
    roll_rate: f32,
    pitch_up_key: KeyCode,
    pitch_down_key: KeyCode,
    roll_left_key: KeyCode,
    roll_right_key: KeyCode,
}

#[derive(Component)]
pub struct CameraAimState {
    pitch: f32,
    roll: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        CameraSettings {
            // fov: 90,
            pos: Vec3::ZERO,
            pitch_rate: 0.01,
            roll_rate: 0.02,
            pitch_up_key: KeyCode::ArrowUp,
            pitch_down_key: KeyCode::ArrowDown,
            roll_left_key: KeyCode::ArrowLeft,
            roll_right_key: KeyCode::ArrowRight,
        }
    }
}

impl Default for CameraAimState {
    fn default() -> Self {
        CameraAimState {
            pitch: 0.0,
            roll: 0.0,
        }
    }
}

pub fn spawn_camera(
    mut commands: Commands,
    my_assets: Res<crate::MyAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut camera = MyCameraBundle::default();
    camera.transform.translation = camera.settings.pos;

    let xhair_mesh = CircularSector::new(0.2, (PI * 3.) / 4.);
    let xhair = meshes.add(xhair_mesh);
    let crosshair = crate::Shape {
        mesh: Mesh3d(xhair),
        material: MeshMaterial3d(my_assets.debug_material.clone()),
        transform: Transform::from_xyz(0., 0., -4.).with_rotation(Quat::from_rotation_z(PI)),
        visibility: Visibility::Visible,
    };

    commands.spawn(camera).with_children(|parent| {
        parent.spawn(crosshair);
    });
}

/// Code to process player input into camera movements
pub fn camera_control(
    kbd: Res<ButtonInput<KeyCode>>,
    // mut evr_motion: EventReader<MouseMotion>,
    mut q_camera: Query<(&CameraSettings, &mut CameraAimState, &mut Transform)>,
) {
    for (cam_settings, cam_state, mut transform) in &mut q_camera {
        let debug_cam_move_l = KeyCode::KeyA;
        let debug_cam_move_r = KeyCode::KeyT;
        let debug_cam_move_u = KeyCode::KeyS;
        let debug_cam_move_d = KeyCode::KeyR;
        let debug_cam_move_f = KeyCode::KeyC;
        let debug_cam_move_b = KeyCode::KeyD;
        let debug_moving_up = kbd.pressed(debug_cam_move_u);
        let debug_moving_down = kbd.pressed(debug_cam_move_d);
        let debug_moving_right = kbd.pressed(debug_cam_move_r);
        let debug_moving_left = kbd.pressed(debug_cam_move_l);
        let debug_moving_fwd = kbd.pressed(debug_cam_move_f);
        let debug_moving_back = kbd.pressed(debug_cam_move_b);

        let pup = cam_settings.pitch_up_key;
        let pdn = cam_settings.pitch_down_key;
        let rleft = cam_settings.roll_left_key;
        let rrt = cam_settings.roll_right_key;
        let pitching_up = kbd.pressed(pup);
        let pitching_down = kbd.pressed(pdn);
        let rolling_right = kbd.pressed(rrt);
        let rolling_left = kbd.pressed(rleft);

        let mut cam_pitch = cam_state.pitch;
        let mut cam_roll = cam_state.roll;
        let mut pitch_total: f32 = 0.0;
        let mut roll_total: f32 = 0.0;

        let mut debug_mov_u_total: f32 = 0.0;
        let mut debug_mov_r_total: f32 = 0.0;
        let mut debug_mov_f_total: f32 = 0.0;
        let debug_mov_rate: f32 = 0.01;
        if debug_moving_up {
            debug_mov_u_total += debug_mov_rate;
        }
        if debug_moving_down {
            debug_mov_u_total -= debug_mov_rate;
        }
        if debug_moving_right {
            debug_mov_r_total += debug_mov_rate;
        }
        if debug_moving_left {
            debug_mov_r_total -= debug_mov_rate;
        }
        if debug_moving_fwd {
            debug_mov_f_total += debug_mov_rate;
        }
        if debug_moving_back {
            debug_mov_f_total -= debug_mov_rate;
        }

        if debug_mov_f_total != 0.0 || debug_mov_r_total != 0.0 || debug_mov_u_total != 0.0 {
            let mov_rt = transform.right() * debug_mov_r_total;
            let mov_up = transform.up() * debug_mov_u_total;
            let mov_fwd = transform.forward() * debug_mov_f_total;

            transform.translation += mov_rt + mov_up + mov_fwd;
        }

        if pitching_up {
            pitch_total += cam_settings.pitch_rate;
        }
        if pitching_down {
            pitch_total -= cam_settings.pitch_rate;
        }
        if rolling_left {
            roll_total += cam_settings.roll_rate;
        }
        if rolling_right {
            roll_total -= cam_settings.roll_rate;
        }

        cam_pitch += pitch_total;
        cam_roll += roll_total;

        if pitch_total != 0.0 || roll_total != 0.0 {
            transform.rotate_local(Quat::from_euler(EulerRot::YXZ, 0.0, cam_pitch, cam_roll));
        }
    }
}

/// Normalize the aim vector so we don't get wonky
pub fn normalize_aim(mut q_camera: Query<&mut Transform, With<CameraSettings>>) {
    if let Ok(mut transform) = q_camera.get_single_mut() {
        transform.rotation = transform.rotation.normalize();
    }
}
