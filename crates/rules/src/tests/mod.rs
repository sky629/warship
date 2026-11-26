//! 규칙 테스트입니다. 화면, 네트워크, 시계 없이 `cargo test -p warship_rules`로 실행합니다.
//!
//! 영역마다 파일 하나입니다. 테스트는 `support::Scenario`로 상황을 만들고, 입력과 `step`으로만 게임을 진행하고,
//! `&Sim`이 공개하는 값(위치, HP, 생명 상태, 무기의 표적)만 확인합니다.

mod support;

mod joining; // 입장: 진영 배정, 정원, 퇴장
mod movement; // 이동: 속도, 목적지에 서는 것, 맵 경계, 멈춤, 가라앉은 배
mod towers; // 타워: 사격, 파괴, 시야
mod tuning; // 기본 수치의 검사와 잘못된 수치의 거절
mod weapons; // 자동 사격: 사거리, 포탄이 닿는 시간, 쿨다운, 시야, 표적 선택, 표적의 종류, 가라앉은 배
