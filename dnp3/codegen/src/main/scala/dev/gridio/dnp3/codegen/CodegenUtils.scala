// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen

import java.nio.charset.Charset
import java.nio.file.{Files, Path, StandardOpenOption}

import dev.gridio.dnp3.codegen.render.*

def writeTo(path: Path)(module: Module): Unit = {

  val content = module.lines(SpacedIndent)

  if(content.isEmpty) {
    throw new Exception(s"Empty file: ${path.toString}")
  }

  val lines = Header() ++ space ++ content


  if(!Files.exists(path.getParent)) Files.createDirectory(path.getParent)
  if(!Files.exists(path)) Files.createFile(path)

  val writer = Files.newBufferedWriter( path, Charset.defaultCharset, StandardOpenOption.TRUNCATE_EXISTING)

  def writeLine(s: String): Unit = {
    writer.write(s)
    writer.write(System.lineSeparator)
  }

  try { lines.foreach(writeLine) }
  finally { writer.close() }
}