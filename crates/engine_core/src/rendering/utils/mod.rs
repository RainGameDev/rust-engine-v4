use ash::vk::{CompareOp, CullModeFlags, Filter, FrontFace, PolygonMode};

pub fn compare_op_to_string(op: CompareOp) -> String {
    match op {
        CompareOp::NEVER => "Never".into(),
        CompareOp::LESS => "Less".into(),
        CompareOp::EQUAL => "Equal".into(),
        CompareOp::LESS_OR_EQUAL => "Less or equal".into(),
        CompareOp::GREATER => "Greater".into(),
        CompareOp::NOT_EQUAL => "Not equal".into(),
        CompareOp::GREATER_OR_EQUAL => "Greater or equal".into(),
        CompareOp::ALWAYS => "Always".into(),
        _ => "Invalid".into(),
    }
}

pub fn filter_to_string(op: Filter) -> String {
    match op {
        Filter::LINEAR => "Linear".into(),
        Filter::NEAREST => "Nearest".into(),
        _ => "Invalid".into(),
    }
}

pub fn polygon_mode_to_string(op: PolygonMode) -> String {
    match op {
        PolygonMode::FILL => "Fill".into(),
        PolygonMode::LINE => "Lines".into(),
        PolygonMode::POINT => "Point".into(),
        _ => "Invalid".into(),
    }
}

pub fn cull_mode_to_string(op: CullModeFlags) -> String {
    match op {
        CullModeFlags::NONE => "None".into(),
        CullModeFlags::FRONT_AND_BACK => "Front and back".into(),
        CullModeFlags::BACK => "Back".into(),
        CullModeFlags::FRONT => "Front".into(),
        _ => "Invalid".into(),
    }
}

pub fn front_face_to_string(op: FrontFace) -> String {
    match op {
        FrontFace::COUNTER_CLOCKWISE => "Counter clockwise".into(),
        FrontFace::CLOCKWISE => "Clockwise".into(),
        _ => "Invalid".into(),
    }
}
