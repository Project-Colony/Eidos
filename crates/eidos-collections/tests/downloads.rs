use eidos_collections::{
    driver::RealHooks,
    install::{Hooks, Obtained},
    manifest::{Mod, Source, SourceType},
};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
};

#[test]
fn a_direct_member_preserves_unrelated_downloads_and_partials() {
    let root =
        std::env::temp_dir().join(format!("eidos-collection-download-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let inst = eidos_instance::Instance::portable(root.join("instance"));
    inst.create().unwrap();
    fs::create_dir_all(inst.downloads_dir()).unwrap();
    let def = eidos_games::catalog()
        .iter()
        .find(|g| g.id == "skyrimse")
        .unwrap();
    let game = eidos_games::DetectedGame {
        source: Default::default(),
        def,
        install_path: root.join("game"),
        data_path: root.join("game/Data"),
        compatdata: None,
        steam_name: "synthetic".into(),
    };
    let nexus = eidos_nexus::Nexus::with_bearer("synthetic-unused");
    let mut say = |_: String| {};
    let mut hooks = RealHooks {
        payload_root: std::path::PathBuf::new(),
        allow_runtime_mismatch: false,
        nexus: &nexus,
        inst: &inst,
        game: &game,
        game_id: def.id.into(),
        say: &mut say,
        collection_domain: def.nexus_game.into(),
        owner: "test:1".into(),
        renamed: vec![],
    };
    for (name, suffix) in [
        ("Complete", ""),
        ("Partial", ".unfinished"),
        ("Sidecar", ".meta"),
    ] {
        let previous = inst.downloads_dir().join(format!("{name}.archive{suffix}"));
        fs::write(&previous, b"personal download bytes").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/member", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap() != 0 {
                request.push(byte[0]);
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nNEW")
                .unwrap();
            String::from_utf8(request).unwrap()
        });
        let member = Mod {
            name: name.into(),
            source: Source {
                kind: SourceType::Direct,
                url,
                file_size: Some(3),
                md5: if name == "Sidecar" {
                    "00000000000000000000000000000000".into()
                } else {
                    String::new()
                },
                ..Default::default()
            },
            ..Default::default()
        };
        let result = hooks.obtain(&member);
        let request = server.join().unwrap();
        assert_eq!(
            fs::read(&previous).unwrap(),
            b"personal download bytes",
            "{name}"
        );
        assert!(
            !request.to_lowercase().contains("range:"),
            "unverified bytes must not be resumed"
        );
        if name == "Sidecar" {
            assert!(
                matches!(result, Obtained::Failed(ref error) if error.contains("MD5 mismatch")),
                "{result:?}"
            );
            assert!(
                fs::read_dir(inst.downloads_dir())
                    .unwrap()
                    .any(|e| fs::read(e.unwrap().path()).is_ok_and(|bytes| bytes == b"NEW")),
                "mismatching downloaded bytes are retained"
            );
            assert_eq!(fs::read_dir(inst.mods_dir()).unwrap().count(), 0);
            continue;
        }
        let Obtained::Ready(path) = result else {
            panic!("{result:?}")
        };
        assert_ne!(path, inst.downloads_dir().join(format!("{name}.archive")));
        assert_eq!(fs::read(path).unwrap(), b"NEW");
    }
    fs::remove_dir_all(root).unwrap();
}
