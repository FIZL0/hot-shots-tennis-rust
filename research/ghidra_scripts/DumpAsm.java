// Print the instructions of the function containing each address argument: DumpAsm.java <out> <hexaddr>...
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.address.*;
import java.io.*;

public class DumpAsm extends GhidraScript {
    @Override public void run() throws Exception {
        String[] a = getScriptArgs();
        try (PrintWriter w = new PrintWriter(a[0])) {
            for (int i = 1; i < a.length; i++) {
                Address at = toAddr(Long.parseLong(a[i], 16));
                Function f = getFunctionContaining(at);
                w.println("== " + (f == null ? a[i] : f.getName() + " " + f.getEntryPoint()));
                InstructionIterator it = currentProgram.getListing().getInstructions(f.getBody(), true);
                while (it.hasNext()) { Instruction ins = it.next(); w.println(ins.getAddress() + "  " + ins); }
            }
        }
    }
}
