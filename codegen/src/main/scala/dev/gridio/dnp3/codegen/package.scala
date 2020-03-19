package dev.gridio.dnp3

import java.nio.charset.Charset
import java.nio.file.{Files, Path, StandardOpenOption}

package object codegen {

  def writeTo(path: Path)(lines: Iterator[String]): Unit = {

    if(lines.isEmpty) {
      throw new Exception(s"Empty file: ${path.toString}")
    }

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

}
