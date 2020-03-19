package dev.gridio.dnp3.codegen.model

import dev.gridio.dnp3.codegen.model.groups._

object ObjectGroup {

  def allVariations : List[GroupVariation] = all.flatMap(g => g.variations)

  val all: List[ObjectGroup] = List(
    Group1,
    Group2,
    Group3,
    Group4,
    Group10,
    Group11,
    Group12,
    Group13,
    Group20,
    Group21,
    Group22,
    Group23,
    Group30,
    Group32,
    Group40,
    Group41,
    Group42,
    Group43,
    Group50,
    Group51,
    Group52,
    Group60,
    /*
    Group70,
    Group80,
    Group110,
    Group111,
    Group112,
    Group113,
    Group120,
    Group121,
    Group122
    */
  )

}

trait ObjectGroup {

  def variations: List[GroupVariation]

  def group: Byte

  def name: String = "Group%s".format(group)

  def desc: String

  def isEventGroup: Boolean

  def hasSizedObjects: Boolean = variations.exists(x => x.isInstanceOf[FixedSizeField])
}
