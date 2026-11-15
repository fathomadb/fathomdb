using System;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
internal static class OwnedChild {
    [StructLayout(LayoutKind.Sequential)] struct Startup {public int Size;public IntPtr A,B,C;public uint D,E,F,G,H,I,J,K;public short L,M;public IntPtr N,O,P,Q;}
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo {public IntPtr Process,Thread;public uint Pid,Tid;}
    [DllImport("kernel32.dll",SetLastError=true,CharSet=CharSet.Unicode)] static extern bool CreateProcessW(string exe,StringBuilder line,IntPtr a,IntPtr b,bool inherit,uint flags,IntPtr env,string cwd,ref Startup startup,out ProcessInfo process);
    [DllImport("kernel32.dll",SetLastError=true)] static extern bool TerminateProcess(IntPtr process,uint code);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr process,uint time);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    public static int Main(string[] args){
        if(args.Length!=2||!Path.GetFullPath(args[1]).StartsWith(@"R:\fathomdb-s20-a6\control\",StringComparison.OrdinalIgnoreCase))return 2;
        string mode=args[0],marker=Path.GetFullPath(args[1]);
        if(mode=="exit"){File.WriteAllText(marker,"entered");return 0;}
        if(mode=="output"){byte[] a=new byte[1024*1024],b=new byte[1024*1024];for(int i=0;i<a.Length;i++){a[i]=(byte)'X';b[i]=(byte)'Y';}using(Stream o=Console.OpenStandardOutput())o.Write(a,0,a.Length);using(Stream e=Console.OpenStandardError())e.Write(b,0,b.Length);return 0;}
        if(mode=="spawn-exit"){ProcessStartInfo si=new ProcessStartInfo(Assembly.GetExecutingAssembly().Location,"linger "+NativeQuote(marker));si.UseShellExecute=false;Process p=Process.Start(si);p.Dispose();return 0;}
        if(mode=="breakaway"){
            Startup si=new Startup();si.Size=Marshal.SizeOf(typeof(Startup));ProcessInfo pi;
            string exe=Assembly.GetExecutingAssembly().Location;
            bool made=CreateProcessW(exe,new StringBuilder(NativeQuote(exe)+" exit "+NativeQuote(marker)),IntPtr.Zero,IntPtr.Zero,false,0x1000000|4,IntPtr.Zero,Path.GetDirectoryName(marker),ref si,out pi);
            if(!made){int error=Marshal.GetLastWin32Error();Console.WriteLine("breakaway-refused-native="+error);return error==5?0:3;}
            // An unexpected breakaway is still suspended. Terminate only the
            // exact process handle returned by this creation, never a sampled PID.
            bool killed=TerminateProcess(pi.Process,125);uint waited=WaitForSingleObject(pi.Process,10000);CloseHandle(pi.Thread);CloseHandle(pi.Process);return killed&&waited==0?4:5;
        }
        if(mode=="linger"){File.WriteAllText(marker,"entered");for(int i=0;i<600;i++){File.WriteAllText(marker+".pulse",i.ToString());Thread.Sleep(200);}return 6;}
        return 2;
    }
    static string NativeQuote(string v){return "\""+v.Replace("\"","\\\"")+"\"";}
}
