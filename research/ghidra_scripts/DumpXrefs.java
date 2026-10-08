// Write every memory reference except branches to <outdir>/<program>.xrefs.tsv: to, from, function at from, type.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.*;
import ghidra.program.model.symbol.*;
import java.io.*;

public class DumpXrefs extends GhidraScript {
    @Override public void run() throws Exception {
        String name = currentProgram.getName().replaceFirst("\\.elf$", "");
        try (PrintWriter w = new PrintWriter(new File(getScriptArgs()[0], name + ".xrefs.tsv"))) {
            for (Reference r : currentProgram.getReferenceManager().getReferenceIterator(currentProgram.getMinAddress())) {
                if (r.getReferenceType().isJump() || !r.getToAddress().isMemoryAddress() || !r.getFromAddress().isMemoryAddress()) continue;
                Function f = getFunctionContaining(r.getFromAddress());
                w.println(r.getToAddress() + "\t" + r.getFromAddress() + "\t" + (f == null ? "-" : f.getName()) + "\t" + r.getReferenceType());
            }
        }
    }
}
