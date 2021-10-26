use anyhow::Context;
use futures::stream::TryStreamExt;
use serde_derive::Deserialize;
use std::collections::HashSet;
use std::fmt;

const BUILDERS: [&'static str; 58] = [
    "aarch64-gnu",
    "arm-android",
    "armhf-gnu",
    "dist-aarch64-apple",
    "dist-aarch64-linux",
    "dist-aarch64-msvc",
    "dist-android",
    "dist-arm-linux",
    "dist-armhf-linux",
    "dist-armv7-linux",
    "dist-i586-gnu-i586-i686-musl",
    "dist-i686-linux",
    "dist-i686-mingw",
    "dist-i686-msvc",
    "dist-mips-linux",
    "dist-mips64-linux",
    "dist-mips64el-linux",
    "dist-mipsel-linux",
    "dist-powerpc-linux",
    "dist-powerpc64-linux",
    "dist-powerpc64le-linux",
    "dist-riscv64-linux",
    "dist-s390x-linux",
    "dist-various-1",
    "dist-various-2",
    "dist-x86_64-apple",
    "dist-x86_64-freebsd",
    "dist-x86_64-illumos",
    "dist-x86_64-linux",
    "dist-x86_64-mingw",
    "dist-x86_64-msvc",
    "dist-x86_64-musl",
    "dist-x86_64-netbsd",
    "i686-gnu-nopt",
    "i686-gnu",
    "i686-mingw-1",
    "i686-mingw-2",
    "i686-msvc-1",
    "i686-msvc-2",
    "mingw-check",
    "test-various",
    "wasm32",
    "x86_64-apple",
    "x86_64-gnu-aux",
    "x86_64-gnu-debug",
    "x86_64-gnu-distcheck",
    "x86_64-gnu-llvm-10",
    "x86_64-gnu-llvm-12",
    "x86_64-gnu-nopt",
    "x86_64-gnu-stable",
    "x86_64-gnu-tools",
    "x86_64-gnu",
    "x86_64-mingw-1",
    "x86_64-mingw-2",
    "x86_64-msvc-1",
    "x86_64-msvc-2",
    "x86_64-msvc-cargo",
    "x86_64-msvc-tools",
];

#[derive(Clone, Deserialize, Debug)]
struct Commit {
    sha: String,
    time: String,
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.sha)
    }
}

// Skip these commits
fn try_load_previous() -> anyhow::Result<HashSet<String>> {
    let mut set = HashSet::new();
    let mut rdr = csv::Reader::from_path("data.csv")?;

    for row in rdr.records() {
        if let Ok(row) = row {
            if let Some(commit) = row.get(0) {
                set.insert(commit.to_owned());
            }
        }
    }

    Ok(set)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let commits: Vec<Commit> = reqwest::get("https://triage.rust-lang.org/bors-commit-list")
        .await?
        .json()
        .await?;

    println!("{} commits", commits.len());

    let seen_commits = try_load_previous().unwrap_or_default();
    if seen_commits.is_empty() {
        let _ = std::fs::remove_file("data.csv");
    }
    let mut output = csv::Writer::from_writer(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("data.csv")?,
    );

    let client = reqwest::Client::new();
    for (idx, commit) in commits.iter().enumerate() {
        if seen_commits.contains(&commit.sha) {
            continue;
        }
        let mut f = futures::stream::FuturesUnordered::new();
        for builder in BUILDERS {
            let client = client.clone();
            f.push(async move {
                let url = format!(
                    "https://ci-artifacts.rust-lang.org/rustc-builds/{}/cpu-{}.csv",
                    commit.sha, builder,
                );
                Ok::<_, anyhow::Error>((
                    builder,
                    client
                        .get(&url)
                        .send()
                        .await
                        .context(builder)?
                        .error_for_status()
                        .context(builder)?
                        .text()
                        .await
                        .context(builder)?,
                ))
            });
        }
        loop {
            match f.try_next().await {
                Ok(Some((builder, csv))) => {
                    let mut rdr = csv::Reader::from_reader(csv.as_bytes());
                    let mut started_at = None;
                    let mut ended_at = None;
                    let mut total_cpu_usage = 0.0;
                    let mut records = 0;
                    for result in rdr.records() {
                        let record =
                            result.with_context(|| format!("record for {} {}", builder, commit))?;
                        let date_time = record.get(0).expect("has time");
                        let date_time = time::PrimitiveDateTime::parse(
                            &format!("{}Z", date_time),
                            &time::format_description::well_known::Rfc3339,
                        )
                        .with_context(|| format!("{} in {} {}", date_time, builder, commit))?;

                        if started_at.is_none() {
                            started_at = Some(date_time);
                        }
                        ended_at = Some(date_time);
                        total_cpu_usage += 100.0 - record.get(1).unwrap().parse::<f64>().unwrap();
                        records += 1;
                    }

                    let (started_at, ended_at) =
                        if let (Some(start), Some(end)) = (started_at, ended_at) {
                            (start, end)
                        } else {
                            anyhow::bail!("Could not find start/end for {} @ {}", builder, commit);
                        };

                    let avg_cpu_usage = total_cpu_usage / (records as f64);

                    output.write_record(&[
                        &commit.sha,
                        &commit.time,
                        builder,
                        &format!(
                            "{}",
                            std::time::Duration::try_from(ended_at - started_at)
                                .unwrap()
                                .as_secs()
                        ),
                        &format!("{:.4}", avg_cpu_usage),
                    ])?;
                }
                Ok(None) => break,
                Err(e) => {
                    if let Some(req) = e.downcast_ref::<reqwest::Error>() {
                        if req.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                            continue;
                        }
                    }
                    eprintln!("{} failed to download: {:?}", commit, e)
                }
            }
        }
        println!("{}/{}", idx, commits.len());
    }

    output.flush()?;

    Ok(())
}
