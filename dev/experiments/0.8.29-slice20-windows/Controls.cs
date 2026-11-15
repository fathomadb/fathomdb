using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Threading;
using System.Web.Script.Serialization;
internal static class Controls {
    static NativeJob active; static bool zero,expectedRefusal; static object nonzeroRoot;
    static void Save(string path,object value){string t=path+".tmp-"+Guid.NewGuid().ToString("N");File.WriteAllText(t,new JavaScriptSerializer().Serialize(value));File.Move(t,path);}
    static bool Empty(NativeJob j,int ms){Stopwatch s=Stopwatch.StartNew();while(s.ElapsedMilliseconds<ms){if(j.ConfirmEmpty().ActiveProcesses==0&&j.EmptyConfirmed)return true;Thread.Sleep(100);}return false;}
    static void Require(bool condition,string message){if(!condition)throw new InvalidOperationException(message);}
    static bool UnexpectedDiagnostics(string mode){
        var snapshot=(Dictionary<string,object>)CleanupDiagnostics.Snapshot();
        if((bool)snapshot["lostOrOverflowed"]&&mode!="diagnostic-overflow"&&mode!="diagnostic-truncation")return true;
        foreach(var entry in (Dictionary<string,object>[])snapshot["entries"]){string operation=(string)entry["operation"];if(operation.StartsWith("injected/"))continue;if(operation=="pre-resume activation guard"&&expectedRefusal)continue;if(operation=="NativeJob construction"&&mode=="inherited-handle")continue;return true;}return false;
    }
    public static int Main(string[] args){
        if(args.Length!=3||!Path.GetFullPath(args[2]).StartsWith(@"R:\fathomdb-s20-a6\control\",StringComparison.OrdinalIgnoreCase))return 2;
        string mode=args[0],child=Path.GetFullPath(args[1]),directory=Path.GetFullPath(args[2]);
        if(!Directory.Exists(directory)||!File.Exists(child))return 2;
        int code=125;string verdict="CONTROL_FAILED",error=null;
        try{verdict=Run(mode,child,directory);code=0;}
        catch(Exception e){error=e.ToString();CleanupDiagnostics.Record("control primary",e,false);}
        finally{
            if(active!=null){
                if(!zero){try{active.Terminate(125);zero=Empty(active,10000);Require(zero,"job empty not confirmed within cleanup bound");}catch(Exception e){CleanupDiagnostics.Record("control cleanup terminate/accounting",e,true);}}
                active.Dispose();
            }
        }
        if(UnexpectedDiagnostics(mode)){code=125;verdict="CONTROL_FAILED_UNEXPECTED_DIAGNOSTIC";}
        Save(Path.Combine(directory,"control-result.json"),new{mode=mode,verdict=verdict,exit=code,authoritativeEmpty=zero,expectedRefusal=expectedRefusal,error=error,diagnostics=CleanupDiagnostics.Snapshot(),nonzeroRoot=nonzeroRoot,runtimeQualification=false});return code;
    }
    static string Run(string mode,string child,string directory){
        string marker=Path.Combine(directory,"child.entered");
        if(mode=="nonzero-root"){
            Type type=Assembly.LoadFrom(Path.Combine(Path.GetDirectoryName(child),"Supervisor.exe")).GetType("Supervisor",true);
            BindingFlags flags=BindingFlags.Static|BindingFlags.NonPublic;
            var fixture=new JavaScriptSerializer().Deserialize<Dictionary<string,object>>(File.ReadAllText(Path.Combine(Path.GetDirectoryName(directory),"run.json")));
            fixture["cargoExe"]=child;fixture["workspace"]=directory;
            type.GetField("cfg",flags).SetValue(null,fixture);type.GetField("control",flags).SetValue(null,directory);
            type.GetField("run",flags).SetValue(null,(string)fixture["runId"]);type.GetField("sha",flags).SetValue(null,(string)fixture["candidateSha"]);
            bool refused=false;
            try{type.GetMethod("Invoke",flags).Invoke(null,new object[]{"nonzero-root",null,30});}
            catch(TargetInvocationException e){refused=e.InnerException is InvalidOperationException;}
            var commands=(List<object>)type.GetField("commands",flags).GetValue(null);
            Require(commands.Count==1,"nonzero root command observation missing");
            var row=(Dictionary<string,object>)commands[0];object counts=row["counts"];
            int exit=Convert.ToInt32(row["rootExit"]);uint count=Convert.ToUInt32(counts.GetType().GetField("ActiveProcesses").GetValue(counts));
            string reason=(string)type.GetField("stop",flags).GetValue(null);
            zero=(bool)row["authoritativeEmpty"]&&count==0;expectedRefusal=refused;
            Require(exit==2&&zero&&refused&&reason!=null&&reason.StartsWith("cargo-exit: nonzero-root=2"),"nonzero root was not refused before empty completion");
            nonzeroRoot=new{actualRootExit=exit,authoritativeEmpty=zero,activeProcesses=count,classification="STOPPED",stopReason=reason,backend=child};
            return "EXPECTED_REFUSAL_NONZERO_ROOT";
        }
        if(mode.StartsWith("owner-")){
            string stage=mode.Substring(6);NativeFaults.Stage=delegate(string s,uint pid,long created){if(s==stage){Save(Path.Combine(directory,"owner-stage.json"),new{stage=s,pid=pid,creation=created,owner=Process.GetCurrentProcess().Id});while(true)Thread.Sleep(100);}};
            active=new NativeJob(child,new[]{"linger",marker},directory,EnvironmentMap(),Path.Combine(directory,"owner.stdout"),Path.Combine(directory,"owner.stderr"));active.Resume();throw new InvalidOperationException("owner stage not reached");
        }
        if(mode.StartsWith("death-")){
            string stage=mode.Substring(6),self=Assembly.GetExecutingAssembly().Location;
            active=new NativeJob(self,new[]{"owner-"+stage,child,directory},directory,EnvironmentMap(),Path.Combine(directory,"death.stdout"),Path.Combine(directory,"death.stderr"));active.Resume();
            string ready=Path.Combine(directory,"owner-stage.json");Stopwatch wait=Stopwatch.StartNew();while(!File.Exists(ready)&&wait.ElapsedMilliseconds<5000)Thread.Sleep(20);Require(File.Exists(ready),"owner stage not reached");
            if(stage=="after-resume"){while(!File.Exists(marker)&&wait.ElapsedMilliseconds<7000)Thread.Sleep(20);Require(File.Exists(marker),"real child did not enter");}else Require(!File.Exists(marker),"suspended/precreate child entered");
            // The outer job is not terminated until this assertion has passed.
            // Its authoritative zero proves the dead owner's inner job closed.
            active.KillRootForControl();zero=Empty(active,10000);Require(zero,"owner death left a contained child alive");return "EXPECTED_OWNED_TERMINATION";
        }
        expectedRefusal=mode=="cleanup-failure"||mode=="diagnostic-overflow"||mode=="diagnostic-truncation"||mode=="inherited-handle";
        if(expectedRefusal)NativeFaults.Mode=mode;
        string childMode=mode=="orphan"?"spawn-exit":mode=="output"?"output":mode=="breakaway"?"breakaway":mode=="linger"?"linger":"exit";
        try{active=new NativeJob(child,new[]{childMode,marker},directory,EnvironmentMap(),Path.Combine(directory,"child.stdout"),Path.Combine(directory,"child.stderr"));active.Resume();}
        catch(Exception e){if(!expectedRefusal)throw;string wanted=mode=="inherited-handle"?"job handle inheritable":"pre-resume setup/cleanup evidence failed or lost: UNKNOWN";Require(e.Message.Contains(wanted),"unrelated failure cannot satisfy refusal control");Require(!File.Exists(marker),"refused child executed");return "EXPECTED_REFUSAL_UNKNOWN";}
        Require(!expectedRefusal,"guard unexpectedly resumed child");
        if(mode=="linger"){while(true)Thread.Sleep(100);}
        if(mode=="orphan"){
            Stopwatch s=Stopwatch.StartNew();while((!active.CaptureRootExit()||!File.Exists(marker))&&s.ElapsedMilliseconds<5000)Thread.Sleep(20);
            Require(active.RootExit==0&&File.Exists(marker),"transient parent/live child not established");Require(active.ReadCounts().ActiveProcesses>0,"orphan escaped accounting");active.Terminate(125);
        }
        zero=Empty(active,10000);Require(zero,"job empty not confirmed");if(mode!="orphan")Require(active.RootExit==0,"child failed");
        if(mode=="output"){byte[] o=File.ReadAllBytes(Path.Combine(directory,"child.stdout")),e=File.ReadAllBytes(Path.Combine(directory,"child.stderr"));Require(o.Length==1048576&&e.Length==1048576,"output truncated");foreach(byte b in o)Require(b==(byte)'X',"stdout altered");foreach(byte b in e)Require(b==(byte)'Y',"stderr altered");}
        return "EXPECTED_OBSERVATION";
    }
    static IDictionary<string,string> EnvironmentMap(){var e=new Dictionary<string,string>(StringComparer.OrdinalIgnoreCase);foreach(System.Collections.DictionaryEntry x in Environment.GetEnvironmentVariables()){string k=(string)x.Key;if(!k.StartsWith("="))e[k]=(string)x.Value;}return e;}
}
