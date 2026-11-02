use current_history_followups::{fixture, history};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let family = &args[0];
    let h: usize = args[1].parse()?;
    let bytes: usize = args[2].parse()?;
    let callers: usize = args[3].parse()?;
    let view = &args[4];
    let root = std::path::Path::new(&args[5]);
    std::fs::create_dir_all(root)?;
    let path = root.join(format!("fixture-{family}-{h}-{bytes}.sqlite"));
    fixture::build(&path, family, h, bytes, 10_000)?;
    let (selection, hydrate) = fixture::queries(family, h, view);
    let next = AtomicUsize::new(0);
    let output = Arc::new(Mutex::new(std::io::stdout()));
    let started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..callers {
            let path = &path;
            let selection = &selection;
            let hydrate = &hydrate;
            let next = &next;
            let output = output.clone();
            scope.spawn(move ||{
            let mut connection=rusqlite::Connection::open_with_flags(path,rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();connection.execute_batch("PRAGMA query_only=ON").unwrap();
            loop {let request=next.fetch_add(1,Ordering::Relaxed);if request>=1000{break;}
                let t=Instant::now();let result=(||->current_history_followups::ledger::Result<history::Search>{let tx=connection.transaction()?;history::search(&tx,selection,hydrate,if request.is_multiple_of(2){"needle"}else{"running alpha"},10,true,||Ok(()))})();
                let record=match result {Ok(result)=>serde_json::json!({"request":request,"status":"success","ms":t.elapsed().as_secs_f64()*1000.,"documents":result.documents,"indexed_bytes":result.indexed_bytes,"winners":result.hits.len(),"memory_temp":result.temp_store==2&&result.private_files.iter().all(String::is_empty)}),Err(error)=>serde_json::json!({"request":request,"status":"error","ms":t.elapsed().as_secs_f64()*1000.,"error":error.to_string()})};
                let mut output=output.lock().unwrap();writeln!(output,"{record}").unwrap();output.flush().unwrap();
            }
        });
        }
    });
    eprintln!("cell finished in {:?}", started.elapsed());
    Ok(())
}
