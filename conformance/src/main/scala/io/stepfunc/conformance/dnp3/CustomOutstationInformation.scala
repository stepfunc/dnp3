// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

package io.stepfunc.conformance.dnp3

import io.stepfunc.dnp3.{BroadcastAction, FunctionCode, OutstationInformation, RequestHeader}
import org.joou.UByte

class CustomOutstationInformation extends OutstationInformation {
  override def processRequestFromIdle(requestHeader: RequestHeader): Unit = {}

  override def broadcastReceived(functionCode: FunctionCode, broadcastAction: BroadcastAction): Unit = {}

  override def enterSolicitedConfirmWait(uByte: UByte): Unit = {}

  override def solicitedConfirmTimeout(uByte: UByte): Unit = {}

  override def solicitedConfirmReceived(uByte: UByte): Unit = {}

  override def solicitedConfirmWaitNewRequest(): Unit = {}

  override def wrongSolicitedConfirmSeq(uByte: UByte, uByte1: UByte): Unit = {}

  override def unexpectedConfirm(b: Boolean, uByte: UByte): Unit = {}

  override def enterUnsolicitedConfirmWait(uByte: UByte): Unit = {}

  override def unsolicitedConfirmTimeout(uByte: UByte, b: Boolean): Unit = {}

  override def unsolicitedConfirmed(uByte: UByte): Unit = {}

  override def clearRestartIin(): Unit = {}
}
