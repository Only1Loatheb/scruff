ThisBuild / scalaVersion := "3.8.4"
ThisBuild / scruffFmtOnCompile := true

// scruff, through a script that logs each run to runsLog.
lazy val runsLog = settingKey[File]("")
ThisBuild / runsLog := (ThisBuild / baseDirectory).value / "target" / "scruff-runs"
ThisBuild / scruffBinary := {
  val wrapper = (ThisBuild / baseDirectory).value / "target" / "scruff-logging"
  IO.write(
    wrapper,
    s"""#!/bin/sh
       |echo "$$*" >> "${runsLog.value}"
       |exec "${sys.props("scruff.binary")}" "$$@"
       |""".stripMargin,
  )
  wrapper.setExecutable(true)
  wrapper
}

lazy val root = (project in file(".")).aggregate(sub1, sub2, sub3)

// Each depends on the one before.
lazy val sub1 = project
lazy val sub2 = project.dependsOn(sub1)
lazy val sub3 = project.dependsOn(sub2)

lazy val off = project.settings(scruffFmtOnCompile := false)

// Whether A.class was compiled again since `recordClass`.
@transient
lazy val recordClass = taskKey[Unit]("")
@transient
lazy val checkNotRecompiled = taskKey[Unit]("")

recordClass := {
  val cls = (Compile / classDirectory).value / "A.class"
  IO.write(target.value / "a-class-stamp", cls.lastModified.toString)
}

checkNotRecompiled := {
  val cls = (Compile / classDirectory).value / "A.class"
  val before = IO.read(target.value / "a-class-stamp")
  if (cls.lastModified.toString != before) sys.error("A.class was compiled again")
}

// `checkRuns n path...`: scruff ran n times since `resetRuns`, on each path.
@transient
lazy val resetRuns = taskKey[Unit]("")
lazy val checkRuns = inputKey[Unit]("")

resetRuns := IO.delete(runsLog.value)

checkRuns := {
  val args = Def.spaceDelimited().parsed
  val runs = if (runsLog.value.isFile) IO.readLines(runsLog.value) else Nil
  if (runs.size != args.head.toInt)
    sys.error(s"scruff ran ${runs.size} times, not ${args.head}: $runs")
  for (path <- args.tail if !runs.exists(_.contains(path)))
    sys.error(s"scruff didn't run on $path: $runs")
}
