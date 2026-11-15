using System;
using System.ComponentModel;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

internal static class CleanupDiagnostics {
    const int Capacity=128, TextLimit=4096;
    static readonly object Gate=new object();
    static readonly List<Dictionary<string,object>> Entries=new List<Dictionary<string,object>>();
    static bool lost,cleanupFailure;
    public static void Record(string operation,Exception exception,bool cleanup) {
        try {
            string text=exception.ToString();bool truncated=text.Length>TextLimit;
            if(truncated)text=text.Substring(0,TextLimit);
            Win32Exception native=exception as Win32Exception;
            lock(Gate){cleanupFailure|=cleanup;if(Entries.Count>=Capacity){lost=true;return;}lost|=truncated;
                Entries.Add(new Dictionary<string,object>{{"operation",operation},{"exceptionType",exception.GetType().FullName},{"exception",text},{"win32NativeErrorCode",native==null?(object)null:native.NativeErrorCode},{"hresult",exception.HResult},{"cleanup",cleanup},{"truncated",truncated},{"utc",DateTime.UtcNow.ToString("o")}});}
        }catch{lock(Gate){lost=true;cleanupFailure|=cleanup;}}
    }
    public static bool Incomplete {get{lock(Gate){return lost;}}}
    public static bool CleanupFailed {get{lock(Gate){return cleanupFailure;}}}
    public static object Snapshot(){lock(Gate){return new Dictionary<string,object>{{"capacity",Capacity},{"textLimit",TextLimit},{"lostOrOverflowed",lost},{"cleanupFailure",cleanupFailure},{"entries",Entries.ToArray()}};}}
}

#if A6_CONTROL
internal static class NativeFaults {
    public static string Mode;
    public static Action<string,uint,long> Stage;
    public static void Reached(string stage,uint pid,long created){if(Stage!=null)Stage(stage,pid,created);}
}
#endif

internal sealed class NativeJob : IDisposable {
    const uint KillOnClose=0x2000, Breakaway=0x800, SilentBreakaway=0x1000;
    const uint Suspended=4, ExtendedStartup=0x80000, UnicodeEnvironment=0x400;
    const uint WaitTimeout=258, WaitFailed=0xffffffff;
    IntPtr job, process, thread;
    readonly List<IntPtr> files=new List<IntPtr>();
    public readonly uint Pid;
    public readonly long CreationFileTime;
    public readonly string CommandLine;
    public bool Resumed {get;private set;}
    public int? RootExit {get;private set;}
    public bool EmptyConfirmed {get;private set;}

    [StructLayout(LayoutKind.Sequential)] struct BasicLimits {
        public long ProcessTime,JobTime; public uint Flags;
        public UIntPtr MinWorkingSet,MaxWorkingSet; public uint ActiveLimit;
        public UIntPtr Affinity; public uint Priority,Scheduling;
    }
    [StructLayout(LayoutKind.Sequential)] struct IoCounters {public ulong A,B,C,D,E,F;}
    [StructLayout(LayoutKind.Sequential)] struct ExtendedLimits {
        public BasicLimits Basic; public IoCounters Io;
        public UIntPtr ProcessMemory,JobMemory,PeakProcessMemory,PeakJobMemory;
    }
    [StructLayout(LayoutKind.Sequential)] public struct Counts {
        public long UserTime,KernelTime,PeriodUserTime,PeriodKernelTime;
        public uint PageFaults,TotalProcesses,ActiveProcesses,TerminatedProcesses;
    }
    [StructLayout(LayoutKind.Sequential)] struct Security {
        public int Length; public IntPtr Descriptor;
        [MarshalAs(UnmanagedType.Bool)] public bool Inherit;
    }
    [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Startup {
        public int Size; public IntPtr Reserved,Desktop,Title;
        public uint X,Y,XSize,YSize,XChars,YChars,Fill,Flags;
        public short Show,ReservedSize; public IntPtr ReservedBytes,Input,Output,Error;
    }
    [StructLayout(LayoutKind.Sequential)] struct StartupEx {public Startup Basic;public IntPtr Attributes;}
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo {public IntPtr Process,Thread;public uint Pid,Tid;}
    [DllImport("kernel32.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern IntPtr CreateJobObjectW(IntPtr security,string name);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetHandleInformation(IntPtr handle,uint mask,uint flags);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetHandleInformation(IntPtr handle,out uint flags);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job,int type,ref ExtendedLimits data,int length);
    [DllImport("kernel32.dll",SetLastError=true,EntryPoint="QueryInformationJobObject")] static extern bool QueryLimits(IntPtr job,int type,out ExtendedLimits data,int length,out int returned);
    [DllImport("kernel32.dll",SetLastError=true,EntryPoint="QueryInformationJobObject")] static extern bool QueryCounts(IntPtr job,int type,out Counts data,int length,out int returned);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool InitializeProcThreadAttributeList(IntPtr list,int count,int flags,ref UIntPtr size);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool UpdateProcThreadAttribute(IntPtr list,uint flags,UIntPtr attribute,IntPtr value,UIntPtr size,IntPtr previous,IntPtr returned);
    [DllImport("kernel32.dll")] static extern void DeleteProcThreadAttributeList(IntPtr list);
    [DllImport("kernel32.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern bool CreateProcessW(string app,StringBuilder command,IntPtr processSecurity,IntPtr threadSecurity,bool inherit,uint flags,IntPtr environment,string cwd,ref StartupEx startup,out ProcessInfo info);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool IsProcessInJob(IntPtr process,IntPtr job,out bool result);
    [DllImport("kernel32.dll",SetLastError=true)] static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool TerminateJobObject(IntPtr job,uint exit);
    [DllImport("kernel32.dll",SetLastError=true)] static extern uint WaitForSingleObject(IntPtr handle,uint milliseconds);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetExitCodeProcess(IntPtr process,out uint code);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetProcessTimes(IntPtr process,out long created,out long exited,out long kernel,out long user);
    [DllImport("kernel32.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern bool QueryFullProcessImageNameW(IntPtr process,uint flags,StringBuilder image,ref int size);
    [DllImport("kernel32.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern IntPtr CreateFileW(string path,uint access,uint share,ref Security security,uint creation,uint flags,IntPtr template);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool CloseHandle(IntPtr handle);

    static void Check(bool ok,string name) {if(!ok){Win32Exception e=new Win32Exception(Marshal.GetLastWin32Error(),name);CleanupDiagnostics.Record(name,e,false);throw e;}}
    static void Close(ref IntPtr value) {if(value!=IntPtr.Zero){IntPtr h=value;value=IntPtr.Zero;Check(CloseHandle(h),"CloseHandle");}}
    static void CloseBestEffort(ref IntPtr value,string operation) {if(value!=IntPtr.Zero){if(!CloseHandle(value))CleanupDiagnostics.Record(operation,new Win32Exception(Marshal.GetLastWin32Error(),operation),true);value=IntPtr.Zero;}}
    static void CloseFileBestEffort(IntPtr value){if(!CloseHandle(value))CleanupDiagnostics.Record("CloseHandle(stdio)",new Win32Exception(Marshal.GetLastWin32Error(),"CloseHandle(stdio)"),true);}
    IntPtr Open(string path,uint access,uint creation) {
        Security s=new Security();s.Length=Marshal.SizeOf(typeof(Security));s.Inherit=true;
        IntPtr h=CreateFileW(path,access,1,ref s,creation,0x80,IntPtr.Zero);
        if(h==new IntPtr(-1))throw new Win32Exception(Marshal.GetLastWin32Error(),"CreateFile "+path);
        files.Add(h);return h;
    }
    public static string Quote(string value) {
        StringBuilder b=new StringBuilder("\"");int slashes=0;
        foreach(char c in value){if(c=='\\'){slashes++;continue;}if(c=='\"'){b.Append('\\',slashes*2+1);b.Append(c);}else{b.Append('\\',slashes);b.Append(c);}slashes=0;}
        b.Append('\\',slashes*2);return b.Append('"').ToString();
    }
    void VerifyLimits() {
        ExtendedLimits got;int size=Marshal.SizeOf(typeof(ExtendedLimits)),returned;
        Check(QueryLimits(job,9,out got,size,out returned),"Query job limits");
        if(returned!=size||got.Basic.Flags!=KillOnClose||(got.Basic.Flags&(Breakaway|SilentBreakaway))!=0)throw new InvalidOperationException("job limits differ");
        uint flags;Check(GetHandleInformation(job,out flags),"Get job inheritance");
        if((flags&1)!=0)throw new InvalidOperationException("job handle inheritable");
    }
    public NativeJob(string exe,string[] args,string cwd,IDictionary<string,string> environment,string stdout,string stderr) {
        IntPtr attributes=IntPtr.Zero,jobList=IntPtr.Zero,handleList=IntPtr.Zero,environmentBlock=IntPtr.Zero;bool attributesReady=false;
        try {
            job=CreateJobObjectW(IntPtr.Zero,null);if(job==IntPtr.Zero)throw new Win32Exception(Marshal.GetLastWin32Error(),"CreateJobObject");
            Check(SetHandleInformation(job,1,0),"Clear job inheritance");
            ExtendedLimits limits=new ExtendedLimits();limits.Basic.Flags=KillOnClose;
            Check(SetInformationJobObject(job,9,ref limits,Marshal.SizeOf(typeof(ExtendedLimits))),"Set job limits");
#if A6_CONTROL
            if(NativeFaults.Mode=="inherited-handle")Check(SetHandleInformation(job,1,1),"injected inheritable handle");
#endif
            VerifyLimits();
            StartupEx startup=new StartupEx();startup.Basic.Size=Marshal.SizeOf(typeof(StartupEx));startup.Basic.Flags=0x100;
            startup.Basic.Input=Open("NUL",0x80000000,3);startup.Basic.Output=Open(stdout,0x40000000,1);startup.Basic.Error=Open(stderr,0x40000000,1);
            UIntPtr bytes=UIntPtr.Zero;
            bool initial=InitializeProcThreadAttributeList(IntPtr.Zero,2,0,ref bytes);
            int initialError=Marshal.GetLastWin32Error();
            if(initial||initialError!=122||bytes==UIntPtr.Zero)throw new Win32Exception(initialError,"Attribute-list size");
            attributes=Marshal.AllocHGlobal(checked((int)bytes.ToUInt64()));
            Check(InitializeProcThreadAttributeList(attributes,2,0,ref bytes),"Initialize attributes");attributesReady=true;
            jobList=Marshal.AllocHGlobal(IntPtr.Size);Marshal.WriteIntPtr(jobList,job);
            Check(UpdateProcThreadAttribute(attributes,0,new UIntPtr(0x2000d),jobList,new UIntPtr((uint)IntPtr.Size),IntPtr.Zero,IntPtr.Zero),"Set creation-time job list");
            handleList=Marshal.AllocHGlobal(3*IntPtr.Size);
            Marshal.WriteIntPtr(handleList,0,startup.Basic.Input);Marshal.WriteIntPtr(handleList,IntPtr.Size,startup.Basic.Output);Marshal.WriteIntPtr(handleList,2*IntPtr.Size,startup.Basic.Error);
            Check(UpdateProcThreadAttribute(attributes,0,new UIntPtr(0x20002),handleList,new UIntPtr((uint)(3*IntPtr.Size)),IntPtr.Zero,IntPtr.Zero),"Set stdio-only handle list");
            startup.Attributes=attributes;
            List<string> keys=new List<string>(environment.Keys);keys.Sort(StringComparer.OrdinalIgnoreCase);
            StringBuilder eb=new StringBuilder();foreach(string key in keys){if(key.IndexOf('=')>=0||key.IndexOf('\0')>=0||environment[key].IndexOf('\0')>=0)throw new InvalidOperationException("invalid environment");eb.Append(key).Append('=').Append(environment[key]).Append('\0');}eb.Append('\0');
            environmentBlock=Marshal.StringToHGlobalUni(eb.ToString());
            StringBuilder line=new StringBuilder(Quote(exe));foreach(string a in args)line.Append(' ').Append(Quote(a));CommandLine=line.ToString();
#if A6_CONTROL
            NativeFaults.Reached("before-create",0,0);
#endif
            ProcessInfo pi;Check(CreateProcessW(exe,line,IntPtr.Zero,IntPtr.Zero,true,Suspended|ExtendedStartup|UnicodeEnvironment,environmentBlock,cwd,ref startup,out pi),"Create contained suspended Cargo");
            process=pi.Process;thread=pi.Thread;Pid=pi.Pid;
            bool inJob;Check(IsProcessInJob(process,job,out inJob),"IsProcessInJob");if(!inJob)throw new InvalidOperationException("root outside private job");VerifyLimits();
            Counts counts=ReadCounts();if(counts.ActiveProcesses!=1)throw new InvalidOperationException("suspended root job count is not1");
            long exit,kernel,user,created;Check(GetProcessTimes(process,out created,out exit,out kernel,out user),"Get root creation time");CreationFileTime=created;
            StringBuilder image=new StringBuilder(32768);int length=image.Capacity;Check(QueryFullProcessImageNameW(process,0,image,ref length),"Query root image");
            if(!String.Equals(Path.GetFullPath(image.ToString()),Path.GetFullPath(exe),StringComparison.OrdinalIgnoreCase))throw new InvalidOperationException("root image identity differs");
#if A6_CONTROL
            NativeFaults.Reached("after-create",Pid,CreationFileTime);
#endif
        } catch(Exception primary) {CleanupDiagnostics.Record("NativeJob construction",primary,false);if(job!=IntPtr.Zero&&!TerminateJobObject(job,125))CleanupDiagnostics.Record("TerminateJobObject(construction cleanup)",new Win32Exception(Marshal.GetLastWin32Error(),"TerminateJobObject(construction cleanup)"),true);Dispose();throw;}
        finally {
            if(attributesReady)DeleteProcThreadAttributeList(attributes);
            if(attributes!=IntPtr.Zero)Marshal.FreeHGlobal(attributes);if(jobList!=IntPtr.Zero)Marshal.FreeHGlobal(jobList);if(handleList!=IntPtr.Zero)Marshal.FreeHGlobal(handleList);if(environmentBlock!=IntPtr.Zero)Marshal.FreeHGlobal(environmentBlock);
            foreach(IntPtr h in files)CloseFileBestEffort(h);files.Clear();
#if A6_CONTROL
            if(NativeFaults.Mode=="cleanup-failure")CleanupDiagnostics.Record("injected/parent-stdio-CloseHandle",new Win32Exception(6,"synthetic API-return fault; no foreign handle is closed"),true);
            if(NativeFaults.Mode=="diagnostic-overflow")for(int i=0;i<129;i++)CleanupDiagnostics.Record("injected/overflow",new Exception("control"),false);
            if(NativeFaults.Mode=="diagnostic-truncation")CleanupDiagnostics.Record("injected/truncation",new Exception(new string('X',4097)),false);
#endif
        }
    }
    public void Resume() {
        VerifyLimits();
        // Constructor-finally cleanup can fail after suspended creation succeeds.
        // Preserve the diagnostic and stop this private job before any activation.
        if(CleanupDiagnostics.Incomplete||CleanupDiagnostics.CleanupFailed){
            InvalidOperationException failure=new InvalidOperationException("pre-resume setup/cleanup evidence failed or lost: UNKNOWN");
            CleanupDiagnostics.Record("pre-resume activation guard",failure,true);
            try{Terminate(125);}catch(Exception e){CleanupDiagnostics.Record("TerminateJobObject(pre-resume guard)",e,true);}
            throw failure;
        }
        uint previous=ResumeThread(thread);
        if(previous!=1)throw new Win32Exception(Marshal.GetLastWin32Error(),"ResumeThread expected suspend count1, got"+previous);
        Resumed=true;Close(ref thread);
#if A6_CONTROL
        NativeFaults.Reached("after-resume",Pid,CreationFileTime);
#endif
    }
#if A6_CONTROL
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool TerminateProcess(IntPtr process,uint code);
    public void KillRootForControl(){Check(TerminateProcess(process,125),"control kill exact owned root handle");}
#endif
    public Counts ReadCounts() {Counts c;int size=Marshal.SizeOf(typeof(Counts)),returned;Check(QueryCounts(job,1,out c,size,out returned),"Query authoritative job accounting");if(returned!=size)throw new InvalidOperationException("accounting length differs");return c;}
    public bool CaptureRootExit() {
        if(process==IntPtr.Zero)return RootExit.HasValue;
        uint wait=WaitForSingleObject(process,0);if(wait==WaitTimeout)return false;if(wait==WaitFailed)throw new Win32Exception(Marshal.GetLastWin32Error(),"Wait root");if(wait!=0)throw new InvalidOperationException("unexpected root wait");
        uint code;Check(GetExitCodeProcess(process,out code),"Get root exit");RootExit=unchecked((int)code);Close(ref process);return true;
    }
    public void Terminate(uint code) {Check(TerminateJobObject(job,code),"Terminate private job");}
    public Counts ConfirmEmpty() {CaptureRootExit();Counts c=ReadCounts();if(c.ActiveProcesses==0){if(!RootExit.HasValue)CaptureRootExit();if(!RootExit.HasValue)throw new InvalidOperationException("empty job without captured root exit");EmptyConfirmed=true;}return c;}
    public void Dispose() {CloseBestEffort(ref thread,"CloseHandle(thread cleanup)");CloseBestEffort(ref process,"CloseHandle(process cleanup)");CloseBestEffort(ref job,"CloseHandle(job cleanup)");foreach(IntPtr h in files)CloseFileBestEffort(h);files.Clear();}
}
