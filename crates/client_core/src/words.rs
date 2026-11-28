//! 서버가 보내는 이름(선체, 무기, 장비, 소모품, 교역품, 스킬, NPC 배, 항구)을 화면에 보여 줄 한국어 이름으로 바꿉니다.
//!
//! 서버의 이름은 영어이고, 에셋 파일의 이름(`assets::slug`)이 그 영어 이름에서 나옵니다. 그래서 서버의 이름은 그대로 두고,
//! 화면에 글자로 보여 줄 때에만 이 표를 거칩니다. 표에 없는 이름은 그대로 보여 줍니다.

/// 영어 이름과 한국어 이름의 표입니다.
const NAMES: &[(&str, &str)] = &[
    // 선체
    ("Sloop", "슬루프"),
    ("Lugger", "러거"),
    ("Cutter", "커터"),
    ("Brig", "브리그"),
    ("Corvette", "코르벳"),
    ("Frigate", "프리깃"),
    ("Tender", "보급함"),
    ("Xebec", "지벡"),
    ("Merchantman", "상선"),
    ("Galleon", "갤리온"),
    ("Bulwark", "방벽함"),
    ("Bomb Ketch", "포격선"),
    ("Man-of-War", "대형 군함"),
    ("Ship of the Line", "전열함"),
    // 무기
    ("Deck Gun", "갑판포"),
    ("Swivel Gun", "선회포"),
    ("Long Nine", "9파운드 장포"),
    ("Carronade", "캐러네이드"),
    ("Mortar", "박격포"),
    ("Twin Deck Guns", "쌍열 갑판포"),
    ("Chain Gun", "연발포"),
    ("Heavy Carronade", "대형 캐러네이드"),
    ("Siege Mortar", "공성 박격포"),
    ("Fire Pot", "화염 단지"),
    ("Culverin", "컬버린"),
    ("Rocket Rack", "로켓 발사대"),
    ("Greek Fire", "그리스의 불"),
    ("Long Twelve", "12파운드 장포"),
    ("Grand Battery", "대포대"),
    // 장비
    ("Oak Planking", "참나무 판재"),
    ("Extra Sails", "추가 돛"),
    ("Shipwrights", "선박 수리공"),
    ("Copper Sheathing", "구리 피복"),
    // 소모품
    ("Repair Kit", "수리 도구"),
    ("Spare Canvas", "예비 돛"),
    ("Sandbags", "모래주머니"),
    // 교역품
    ("Timber", "목재"),
    ("Salt", "소금"),
    ("Spice", "향신료"),
    ("Silk", "비단"),
    // 스킬
    ("Patch Up", "응급 수리"),
    ("Grapeshot", "포도탄"),
    ("Full Sail", "전속 항해"),
    ("Broadside", "일제 사격"),
    ("Iron Hull", "강철 선체"),
    ("Field Repairs", "함대 수리"),
    ("Chain Shot", "사슬탄"),
    ("Night Run", "야간 잠행"),
    // NPC 배
    ("Sea Raider", "해적선"),
    // 항구
    ("Base and harbors", "본진과 항구"),
    ("North Port", "북쪽 항구"),
    ("South Port", "남쪽 항구"),
    ("Gunsmith Isle", "대장장이 섬"),
    ("Rigging Isle", "돛 장인의 섬"),
];

/// 서버가 보낸 이름의 한국어 이름입니다. 표에 없으면 받은 이름 그대로입니다.
pub fn ko(name: &str) -> &str {
    let found = NAMES.iter().find(|(english, _)| *english == name);
    found.map_or(name, |(_, korean)| korean)
}

#[cfg(test)]
mod tests {
    use warship_rules::Tuning;

    use super::ko;

    #[test]
    fn every_name_the_server_sends_has_a_korean_name() {
        let tuning = Tuning::default();
        let mut names: Vec<&str> = Vec::new();
        names.extend(tuning.hulls.iter().map(|hull| hull.name));
        names.extend(tuning.weapons.iter().map(|weapon| weapon.name));
        names.extend(tuning.skills.iter().map(|skill| skill.name));
        names.extend(tuning.gear.iter().map(|gear| gear.name));
        names.extend(tuning.supplies.iter().map(|supply| supply.name));
        names.extend(tuning.trades.iter().map(|trade| trade.name));
        names.extend(tuning.creeps.iter().map(|creep| creep.name));
        names.extend(tuning.map.ports.iter().map(|port| port.name));
        let missing: Vec<&str> = names.into_iter().filter(|name| ko(name) == *name).collect();
        assert_eq!(missing, Vec::<&str>::new());
    }

    #[test]
    fn a_name_outside_the_table_is_shown_as_it_is() {
        assert_eq!(ko("Sloop"), "슬루프");
        assert_eq!(ko("Unknown Thing"), "Unknown Thing");
    }
}
