
pub fn seconds_to_music_time(total_seconds: i64) -> String {
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{:0}:{:02}", minutes, seconds)
}