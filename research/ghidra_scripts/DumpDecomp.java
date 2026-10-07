// Decompile every function to <outdir>/<program>.c and list functions in <program>.funcs.tsv
import ghidra.app.script.GhidraScript;
import ghidra.app.decompiler.*;
import ghidra.program.model.listing.*;
import java.io.*;

public class DumpDecomp extends GhidraScript {
    @Override public void run() throws Exception {
        String out = getScriptArgs()[0];
        String name = currentProgram.getName().replace(".elf", "");
        DecompInterface d = new DecompInterface();
        d.openProgram(currentProgram);
        try (PrintWriter c = new PrintWriter(out + "/" + name + ".c");
             PrintWriter t = new PrintWriter(out + "/" + name + ".funcs.tsv")) {
            for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
                if (monitor.isCancelled()) break;
                DecompileResults r = d.decompileFunction(f, 60, monitor);
                String code = r.decompileCompleted() ? r.getDecompiledFunction().getC() : "/* decompile failed: " + r.getErrorMessage() + " */\n";
                c.println("// " + f.getEntryPoint() + " size=" + f.getBody().getNumAddresses());
                c.println(code);
                t.println(f.getEntryPoint() + "\t" + f.getName() + "\t" + f.getBody().getNumAddresses() + "\t" + f.getCallingFunctions(monitor).size());
            }
        }
    }
}
