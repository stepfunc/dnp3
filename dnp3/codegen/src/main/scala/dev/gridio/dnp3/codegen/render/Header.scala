// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package dev.gridio.dnp3.codegen.render

object Header {

  def apply(): Iterator[String] = {
    license.iterator.map(s => s"// $s") ++ Iterator("") ++ list.iterator.map(s => if(s.isEmpty) "//" else s"// $s")
  }

  private val license = List(
    "Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.",
    "No use to design, develop, specify, test, validate, or improve Other Implementations,",
    "subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.",
    "Text and data mining rights, including for AI training, are reserved under",
    "Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.",
  )

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
