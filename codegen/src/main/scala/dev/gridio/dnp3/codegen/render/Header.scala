package dev.gridio.dnp3.codegen.render

object Header {

  def apply(): Iterator[String] = {
    list.iterator.map(s => if(s.isEmpty) "//" else s"// $s")
  }

  private val list = List(
    """ _   _         ______    _ _ _   _             _ _ _""",
    """| \ | |       |  ____|  | (_) | (_)           | | | |""",
    """|  \| | ___   | |__   __| |_| |_ _ _ __   __ _| | | |""",
    """| . ` |/ _ \  |  __| / _` | | __| | '_ \ / _` | | | |""",
    """| |\  | (_) | | |___| (_| | | |_| | | | | (_| |_|_|_|""",
    """|_| \_|\___/  |______\__,_|_|\__|_|_| |_|\__, (_|_|_)""",
    """                                          __/ |""",
    """                                         |___/""",
    "",
    """This file is auto-generated. Do not edit manually""",
    "",
  )
}
