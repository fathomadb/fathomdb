using System;
using System.Collections;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Management;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Web.Script.Serialization;

internal static class Supervisor {
    const long GiB=1073741824L;
    static readonly object Sync=new object();
    static readonly Stopwatch Clock=Stopwatch.StartNew();
    static JavaScriptSerializer Serializer(){return new JavaScriptSerializer {MaxJsonLength=134217728};}
    static Dictionary<string,object> cfg;
    static string control,run,sha,stop;
    static bool unknown;
    static volatile bool done;
    static NativeJob active;
    static long leaseAt,resourceAt,generation=-1,stopAt=-1;
    static Dictionary<string,object> resources;
    static readonly List<object> commands=new List<object>();
    [DllImport("kernel32.dll")] static extern void ExitProcess(uint code);
    static string S(IDictionary<string,object> d,string key) {object v;if(!d.TryGetValue(key,out v)||!(v is string)||String.IsNullOrEmpty((string)v))throw new InvalidDataException("missing string "+key);return (string)v;}
    static string C(string key) {return S(cfg,key);}
    static Dictionary<string,object> Read(string path) {using(FileStream f=new FileStream(path,FileMode.Open,FileAccess.Read,FileShare.ReadWrite|FileShare.Delete))using(StreamReader r=new StreamReader(f,Encoding.UTF8)){return Serializer().Deserialize<Dictionary<string,object>>(r.ReadToEnd());}}
    static Dictionary<string,object> ReadMutable(string path) {
        FileStream f;
        try{f=new FileStream(path,FileMode.Open,FileAccess.Read,FileShare.ReadWrite|FileShare.Delete);}
        catch(IOException e){int code=e.HResult&0xffff;if(code==2||code==32||code==33)return null;throw;}
        using(f)using(StreamReader r=new StreamReader(f,Encoding.UTF8)){return Serializer().Deserialize<Dictionary<string,object>>(r.ReadToEnd());}
    }
    static string Hash(string path) {using(SHA256 h=SHA256.Create())using(FileStream f=File.OpenRead(path)){return BitConverter.ToString(h.ComputeHash(f)).Replace("-","").ToLowerInvariant();}}
    static string Owned(string path) {string full=Path.GetFullPath(path),root=Path.GetFullPath(C("root")).TrimEnd('\\')+"\\";if(!full.StartsWith(root,StringComparison.OrdinalIgnoreCase))throw new InvalidDataException("path outside owned R root");return full;}
    static void Atomic(string path,object value) {
        string temp=path+".tmp-"+Guid.NewGuid().ToString("N");
        using(FileStream f=new FileStream(temp,FileMode.CreateNew,FileAccess.Write,FileShare.None))using(StreamWriter w=new StreamWriter(f,new UTF8Encoding(false))){w.Write(Serializer().Serialize(value));w.Write('\n');w.Flush();f.Flush(true);}
        if(File.Exists(path))File.Replace(temp,path,null);else File.Move(temp,path);
    }
    static Dictionary<string,object> Record(string phase) {return new Dictionary<string,object>{{"runId",run},{"candidateSha",sha},{"phase",phase},{"utc",DateTime.UtcNow.ToString("o")},{"supervisorPid",Process.GetCurrentProcess().Id}};}
    static void Stop(string reason,bool uncertain) {
        lock(Sync){if(stop==null){stop=reason;stopAt=Clock.ElapsedMilliseconds;}unknown|=uncertain;if(active!=null){try{active.Terminate(125);}catch(Exception e){CleanupDiagnostics.Record("TerminateJobObject(Stop)",e,true);unknown=true;}}}
    }
    static void NeedRunning() {lock(Sync){if(stop!=null)throw new InvalidOperationException("stop: "+stop);}}
    static void CheckIdentity(Dictionary<string,object> value) {if(S(value,"runId")!=run||S(value,"candidateSha")!=sha)throw new InvalidDataException("run/source identity mismatch");}
    static void LeaseReader() {
        try {
            while(!done) {
                string cancel=Path.Combine(control,"cancel.json");
                if(File.Exists(cancel)){Dictionary<string,object> value=ReadMutable(cancel);if(value==null){Thread.Sleep(250);continue;}CheckIdentity(value);Stop("explicit-host-cancel",false);return;}
                Dictionary<string,object> l=ReadMutable(Path.Combine(control,"lease.json"));if(l==null){Thread.Sleep(250);continue;}CheckIdentity(l);
                object raw;if(!l.TryGetValue("generation",out raw)||(raw is bool)||!(raw is int||raw is long))throw new InvalidDataException("invalid lease generation");
                long next=Convert.ToInt64(raw);
                lock(Sync){if(next<generation)throw new InvalidDataException("regressed lease");if(next>generation){generation=next;leaseAt=Clock.ElapsedMilliseconds;}}
                Thread.Sleep(250);
            }
        } catch(Exception e){CleanupDiagnostics.Record("lease reader",e,false);Stop("lease-error: "+e.Message,true);}
    }
    static ManagementObject One(string scope,string query) {
        ConnectionOptions connection=new ConnectionOptions();connection.Timeout=TimeSpan.FromSeconds(2);
        ManagementScope ms=new ManagementScope(scope,connection);ms.Connect();
        EnumerationOptions options=new EnumerationOptions();options.Timeout=TimeSpan.FromSeconds(2);options.ReturnImmediately=false;
        using(ManagementObjectSearcher search=new ManagementObjectSearcher(ms,new ObjectQuery(query),options))using(ManagementObjectCollection result=search.Get()) {
            ManagementObject chosen=null;int count=0;
            foreach(ManagementObject value in result){count++;if(chosen!=null)chosen.Dispose();chosen=value;}
            if(count!=1){if(chosen!=null)chosen.Dispose();throw new InvalidDataException("ambiguous WMI resource query");}
            return chosen;
        }
    }
    static Dictionary<string,object> Measure() {
        DriveInfo c=new DriveInfo("C"),r=new DriveInfo("R");
        if(!r.IsReady||r.DriveFormat!="NTFS"||r.VolumeLabel!="FDB29S20A6")throw new InvalidDataException("R volume differs");
        ulong memory;
        using(ManagementObject os=One(@"\\.\root\cimv2","SELECT FreePhysicalMemory FROM Win32_OperatingSystem")){memory=Convert.ToUInt64(os["FreePhysicalMemory"])*1024UL;}
        using(ManagementObject p=One(@"\\.\root\Microsoft\Windows\Storage","SELECT DiskNumber,Guid FROM MSFT_Partition WHERE DriveLetter=82")) {
            if(Guid.Parse(Convert.ToString(p["Guid"]))!=Guid.Parse(C("partitionGuid")))throw new InvalidDataException("R partition differs");
            uint disk=Convert.ToUInt32(p["DiskNumber"]);
            using(ManagementObject d=One(@"\\.\root\Microsoft\Windows\Storage","SELECT UniqueId,SerialNumber,IsBoot,IsSystem,IsOffline,IsReadOnly,Size FROM MSFT_Disk WHERE Number="+disk)) {
                if(disk==0||Convert.ToString(d["UniqueId"])!=C("diskUniqueId")||Convert.ToString(d["SerialNumber"]).Trim()!="FDB29S20A6"||Convert.ToBoolean(d["IsBoot"])||Convert.ToBoolean(d["IsSystem"])||Convert.ToBoolean(d["IsOffline"])||Convert.ToBoolean(d["IsReadOnly"])||Convert.ToUInt64(d["Size"])!=68719476736UL)throw new InvalidDataException("owned disk identity differs");
            }
        }
        return new Dictionary<string,object>{{"cFree",c.AvailableFreeSpace},{"rFree",r.AvailableFreeSpace},{"memoryAvailable",memory},{"partitionGuid",C("partitionGuid")},{"diskUniqueId",C("diskUniqueId")},{"measuredUtc",DateTime.UtcNow.ToString("o")}};
    }
    static void ResourceReader() {
        try {
            while(!done){Dictionary<string,object> sample=Measure();lock(Sync){resources=sample;resourceAt=Clock.ElapsedMilliseconds;}if(Convert.ToInt64(sample["cFree"])<5*GiB||Convert.ToInt64(sample["rFree"])<20*GiB||Convert.ToUInt64(sample["memoryAvailable"])<8UL*(ulong)GiB){Stop("guest-resource-floor",false);return;}Thread.Sleep(1000);}
        }catch(Exception e){CleanupDiagnostics.Record("resource reader",e,false);Stop("guest-resource-identity-error: "+e.Message,true);}
    }
    static void Watchdog() {
        try {
            while(!done){
                long now=Clock.ElapsedMilliseconds,stopped;
                lock(Sync){stopped=stopAt;if(generation>=0&&now-leaseAt>=60000)Stop("lease-expired",false);if(resources!=null&&now-resourceAt>3000)Stop("resource-sample-stale",true);if(now>=1680000)Stop("overall-timeout",false);if(resources==null&&now>6000)Stop("resource-start-unavailable",true);if(generation<0&&now>6000)Stop("initial-lease-unavailable",true);stopped=stopAt;}
                // This independent thread also covers main-thread filesystem stalls.
                // Forced process exit closes the sole private job handle. Without
                // the main thread's authoritative empty receipt the host stays UNKNOWN.
                if(stopped>=0&&now-stopped>22000){Stop("supervisor-cleanup-bound",true);ExitProcess(125);}
                Thread.Sleep(200);
            }
        }catch(Exception e){CleanupDiagnostics.Record("watchdog",e,true);Stop("watchdog-error",true);ExitProcess(125);}
    }
    static void StartThread(ThreadStart body){Thread t=new Thread(body);t.IsBackground=true;t.Start();}
    static void VerifyInputs(string expectedConfigHash) {
        if(!Regex.IsMatch(run,@"\A[0-9a-f]{32}\z")||!Regex.IsMatch(sha,@"\A[0-9a-f]{40}\z"))throw new InvalidDataException("run/source syntax");
        if(Path.GetFullPath(C("root")).TrimEnd('\\')!=@"R:\fathomdb-s20-a6")throw new InvalidDataException("root differs");
        if(!String.Equals(control,Path.Combine(C("root"),"control",run),StringComparison.OrdinalIgnoreCase))throw new InvalidDataException("control path differs");
        string executable=Assembly.GetExecutingAssembly().Location;
        if(Hash(executable)!=C("wrapperSha256"))throw new InvalidDataException("wrapper binary hash differs");
        if(Hash(Owned(C("sourceArchive")))!=C("archiveSha256"))throw new InvalidDataException("source archive digest differs");
        if(Hash(C("cargoExe"))!=C("cargoSha256"))throw new InvalidDataException("Cargo executable digest differs");
        if(Hash(C("rustcExe"))!=C("rustcSha256"))throw new InvalidDataException("Rust compiler digest differs");
        string workspace=Owned(C("workspace"));
        if(!String.Equals(workspace,Path.Combine(C("root"),"source",sha),StringComparison.OrdinalIgnoreCase))throw new InvalidDataException("workspace differs");
        Dictionary<string,object> sourceFiles=cfg["sourceFiles"] as Dictionary<string,object>;if(sourceFiles==null||sourceFiles.Count==0)throw new InvalidDataException("source inventory missing");
        HashSet<string> expected=new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach(KeyValuePair<string,object> file in sourceFiles){NeedRunning();string relative=file.Key.Replace('/', '\\');if(Path.IsPathRooted(relative)||relative.Split('\\').Length==0||Array.IndexOf(relative.Split('\\'),"..")>=0)throw new InvalidDataException("unsafe source entry");string path=Path.GetFullPath(Path.Combine(workspace,relative));if(!path.StartsWith(workspace.TrimEnd('\\')+"\\",StringComparison.OrdinalIgnoreCase)||!expected.Add(path))throw new InvalidDataException("source entry escape/duplicate");if((File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0||Hash(path)!=(string)file.Value)throw new InvalidDataException("source content differs: "+relative);}
        foreach(string directory in Directory.GetDirectories(workspace,"*",SearchOption.AllDirectories)){NeedRunning();if((File.GetAttributes(directory)&FileAttributes.ReparsePoint)!=0)throw new InvalidDataException("source directory is reparse point");}
        string[] actual=Directory.GetFiles(workspace,"*",SearchOption.AllDirectories);if(actual.Length!=expected.Count)throw new InvalidDataException("unexpected source file count");foreach(string path in actual)if(!expected.Contains(Path.GetFullPath(path)))throw new InvalidDataException("unexpected source file");
        Dictionary<string,object> receipt=Record("supervisor-start");receipt["configSha256"]=expectedConfigHash;receipt["wrapperSha256"]=C("wrapperSha256");receipt["archiveSha256"]=C("archiveSha256");receipt["sourceFileCount"]=expected.Count;receipt["readyGeneration"]=generation;receipt["executable"]=executable;receipt["creationUtc"]=Process.GetCurrentProcess().StartTime.ToUniversalTime().ToString("o");Atomic(Path.Combine(control,"supervisor.json"),receipt);
    }
    static IDictionary<string,string> EnvironmentForChild(){Dictionary<string,string> e=new Dictionary<string,string>(StringComparer.OrdinalIgnoreCase);foreach(DictionaryEntry value in Environment.GetEnvironmentVariables()){string key=(string)value.Key;if(!key.StartsWith("="))e[key]=(string)value.Value;}string root=C("root");e["RUSTC"]=C("rustcExe");e["CARGO_HOME"]=Path.Combine(root,"cargo-home");e["CARGO_TARGET_DIR"]=Path.Combine(root,"target");e["CARGO_BUILD_JOBS"]="2";e["CMAKE_BUILD_PARALLEL_LEVEL"]="2";e["TEMP"]=Path.Combine(root,"tmp");e["TMP"]=e["TEMP"];e["CARGO_TERM_COLOR"]="never";return e;}
    static void Invoke(string label,string selector,int timeoutSeconds) {
        NeedRunning();
        string outFile=Path.Combine(control,label+".stdout.log"),errFile=Path.Combine(control,label+".stderr.log");
        List<string> args=new List<string>{"test","--locked","--manifest-path",Path.Combine(C("workspace"),"Cargo.toml"),"--target-dir",Path.Combine(C("root"),"target"),"--jobs","2","-p","fathomdb-engine","--features","operator,test-hooks","--lib"};
        if(selector==null)args.Add("--no-run");else{args.Add(selector);args.Add("--");args.Add("--exact");args.Add("--nocapture");}
        NativeJob j=null;bool zero=false,terminated=false;long rootExitAt=-1,cleanupAt=-1;NativeJob.Counts counts=new NativeJob.Counts();Stopwatch commandClock=Stopwatch.StartNew();
        Dictionary<string,object> item=Record(label);item["stdout"]=outFile;item["stderr"]=errFile;item["selector"]=selector;item["timeoutSeconds"]=timeoutSeconds;
        try {
            j=new NativeJob(C("cargoExe"),args.ToArray(),C("workspace"),EnvironmentForChild(),outFile,errFile);
            lock(Sync){active=j;NeedRunning();item["rootPid"]=j.Pid;item["rootCreationFileTime"]=j.CreationFileTime;item["commandLine"]=j.CommandLine;item["jobLimits"]=0x2000;item["creationTimeJobAssignment"]=true;}
            Atomic(Path.Combine(control,label+".created.json"),item);
            lock(Sync){NeedRunning();j.Resume();}
            while(true){
                lock(Sync){if(commandClock.Elapsed.TotalSeconds>=timeoutSeconds)Stop("command-timeout: "+label,false);if(stop!=null&&!terminated){j.Terminate(125);terminated=true;cleanupAt=commandClock.ElapsedMilliseconds;}}
                counts=j.ConfirmEmpty();bool exited=j.RootExit.HasValue;
                if(exited&&rootExitAt<0){rootExitAt=commandClock.ElapsedMilliseconds;if(j.RootExit.Value!=0)Stop("cargo-exit: "+label+"="+j.RootExit.Value,false);}
                if(counts.ActiveProcesses==0&&j.EmptyConfirmed){zero=true;break;}
                long now=commandClock.ElapsedMilliseconds;
                if(exited&&!terminated&&now-rootExitAt>=10000){Stop("descendant-not-quiescent: "+label,false);}
                if(terminated&&now-cleanupAt>=10000){unknown=true;throw new TimeoutException("job did not confirm empty within cleanup bound");}
                Thread.Sleep(200);
            }
            item["rootExit"]=j.RootExit;item["counts"]=counts;item["authoritativeEmpty"]=true;item["emptyUtc"]=DateTime.UtcNow.ToString("o");item["lastLeaseGeneration"]=generation;item["stopReason"]=stop;item["diagnostics"]=CleanupDiagnostics.Snapshot();
            Atomic(Path.Combine(control,label+".empty.json"),item);commands.Add(item);
            if(selector!=null&&j.RootExit==0&&stop==null){string output=File.ReadAllText(outFile,Encoding.UTF8);if(!Regex.IsMatch(output,@"test result: ok\. 1 passed; 0 failed; 0 ignored;"))Stop("selected-count-not-exactly-one: "+label,false);}
            NeedRunning();
        }catch(Exception e){Stop("invocation-error: "+label+": "+e.Message,!zero);throw;}
        finally {lock(Sync){if(active==j)active=null;if(j!=null)j.Dispose();}if(!zero)unknown=true;if(CleanupDiagnostics.Incomplete||CleanupDiagnostics.CleanupFailed)Stop("cleanup-diagnostics-incomplete-or-failed",true);}
    }
    public static int Main(string[] args) {
        try {
            if(args.Length!=2||!Regex.IsMatch(args[1],@"\A[0-9a-f]{64}\z"))throw new ArgumentException("Supervisor.exe CONTROL_DIRECTORY CONFIG_SHA256");
            control=Path.GetFullPath(args[0]);if(!Regex.IsMatch(control,@"\AR:\\fathomdb-s20-a6\\control\\[0-9a-f]{32}\z",RegexOptions.IgnoreCase))throw new InvalidDataException("control directory syntax");string config=Path.Combine(control,"run.json");if(Hash(config)!=args[1])throw new InvalidDataException("run manifest digest differs");cfg=Read(config);run=C("runId");sha=C("candidateSha");
            StartThread(LeaseReader);StartThread(ResourceReader);StartThread(Watchdog);
            while(true){NeedRunning();lock(Sync){if(resources!=null&&generation>=0)break;}Thread.Sleep(50);}
            lock(Sync){if(Convert.ToInt64(resources["cFree"])<8*GiB||Convert.ToInt64(resources["rFree"])<32*GiB||Convert.ToUInt64(resources["memoryAvailable"])<16UL*(ulong)GiB)throw new InvalidOperationException("guest start floor");}
            VerifyInputs(args[1]);
            long readyGeneration;lock(Sync){readyGeneration=generation;}
            // Require a fresh host renewal after source/identity readiness.
            while(true){NeedRunning();lock(Sync){if(generation>readyGeneration)break;}Thread.Sleep(50);}
            foreach(string owned in new[]{"cargo-home","target","tmp"})Directory.CreateDirectory(Owned(Path.Combine(C("root"),owned)));
            Invoke("build",null,1200);
            Invoke("startup","slice90_close_review_tests::failed_startup_after_accepted_probe_preserves_error_and_cleans_sqlite_profiles",90);
            Invoke("verification","slice90_post_probe_real_error_tests::verification_timeout_precedes_later_visibility_fault_and_retains_provider_only",90);
            Invoke("visibility","slice90_post_probe_real_error_tests::accepted_probe_still_preserves_later_visibility_error_and_releases_database_owners",90);
            Invoke("opener","tests::current_opener_holds_lock_before_admission_classification",90);
            NeedRunning();Dictionary<string,object> final=Record("COMPLETED");final["commands"]=commands;final["lastLeaseGeneration"]=generation;final["resources"]=resources;final["diagnostics"]=CleanupDiagnostics.Snapshot();if(CleanupDiagnostics.Incomplete||CleanupDiagnostics.CleanupFailed)throw new InvalidOperationException("cleanup evidence incomplete");Atomic(Path.Combine(control,"termination.json"),final);done=true;return 0;
        } catch(Exception e) {
            CleanupDiagnostics.Record("supervisor",e,false);Stop("supervisor-error: "+e.ToString(),CleanupDiagnostics.Incomplete||CleanupDiagnostics.CleanupFailed);
            try {if(control!=null&&run!=null){Dictionary<string,object> final=Record(unknown||CleanupDiagnostics.Incomplete||CleanupDiagnostics.CleanupFailed||commands.Count==0?"UNKNOWN":"STOPPED");final["reason"]=stop;final["commands"]=commands;final["lastLeaseGeneration"]=generation;final["rawError"]=e.ToString();final["diagnostics"]=CleanupDiagnostics.Snapshot();Atomic(Path.Combine(control,"termination.json"),final);}}catch(Exception receiptError){CleanupDiagnostics.Record("termination receipt persistence",receiptError,true);unknown=true;}
            done=true;return 125;
        }
    }
}
