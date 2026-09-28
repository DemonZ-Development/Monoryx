#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextStep {
    Mods,
    Settings,
    Repair,
    Logs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advice {
    pub title: String,
    pub explanation: String,
    pub steps: Vec<String>,
    pub evidence: Vec<String>,
    pub next: NextStep,
}

pub fn explain(log: &str, _exit_code: i32) -> Advice {
    let lines = messages(log);
    let clean = lines.join("\n");
    let lower = clean.to_lowercase();
    let dependency: Vec<_> = lines
        .iter()
        .filter(|line| {
            (line.contains(" requires ")
                && (line.contains("missing")
                    || line.contains("wrong version")
                    || line.contains("can't be loaded")))
                || line.contains("HARD_DEP_NO_CANDIDATE")
                || line.contains(" is incompatible with ")
        })
        .cloned()
        .collect();
    if !dependency.is_empty()
        || lower.contains("incompatible mods found")
        || lower.contains("mod resolution failed")
    {
        let mut result = advice("Some mods don't work together", "The mod loader could not find a compatible set of mods for this instance.", NextStep::Mods,
            &["Check the requirements listed below, then install the matching mod or dependency version.", "Make sure each download matches this instance's Minecraft version and mod loader."]);
        result.evidence = dependency
            .iter()
            .filter(|line| !line.contains("HARD_DEP"))
            .take(3)
            .cloned()
            .collect();
        if result.evidence.is_empty() {
            for line in &dependency {
                if let Some(rest) = line.split("HARD_DEP_NO_CANDIDATE ").nth(1) {
                    if let Some((owner, required)) = rest.split_once("{depends ") {
                        if let Some((name, version)) = required.split_once(" @ ") {
                            let range = version.split('}').next().unwrap_or(version).trim();
                            result.evidence.push(format!(
                                "{} needs {} {}. A compatible enabled copy wasn't found.",
                                owner.trim(),
                                friendly_mod(name),
                                range
                            ));
                        }
                    }
                }
                if result.evidence.len() >= 3 {
                    break;
                }
            }
        }
        if result.evidence.len() == 1 {
            result.explanation = result.evidence[0].clone();
        }
        return result;
    }
    let (title, explanation, next, steps): (&str, &str, NextStep, &[&str]) = if lower
        .contains("outofmemoryerror")
    {
        ("Minecraft ran out of memory", "Java reported that it could not allocate enough memory to continue.", NextStep::Settings,
         &["Close unused apps and check the memory limit for this instance.", "If your computer has spare memory, raise the limit a little. Keep memory available for Windows and other apps."])
    } else if lower.contains("could not reserve enough space")
        || lower.contains("invalid maximum heap")
        || lower.contains("invalid initial heap")
    {
        (
            "Java couldn't use the memory setting",
            "The requested amount of memory is unavailable or invalid.",
            NextStep::Settings,
            &[
                "Lower the memory limit or close other apps, then try again.",
                "Use a 64-bit Java runtime and check any custom memory flags.",
            ],
        )
    } else if lower.contains("unsupportedclassversionerror") {
        (
            "The Java version doesn't match",
            "A game or mod file requires a different Java version from the one being used.",
            NextStep::Settings,
            &[
                "Choose Automatic for Java in this instance's settings.",
                "Check that your mods were made for this Minecraft version.",
            ],
        )
    } else if lower.contains("unrecognized vm option")
        || lower.contains("could not create the java virtual machine")
    {
        (
            "Java couldn't start",
            "A custom Java option or memory setting may be preventing startup.",
            NextStep::Settings,
            &[
                "Check the error below for an unsupported Java option.",
                "Review custom Java arguments and memory settings, then try again.",
            ],
        )
    } else if lower.contains("mixinapplyerror")
        || lower.contains("mixintransformererror")
        || lower.contains("mixin apply failed")
    {
        (
            "A mod couldn't load",
            "The log reports a mod patching error. This can happen with incompatible mod versions.",
            NextStep::Mods,
            &[
                "Check the mod named in the error and its required dependencies.",
                "Try disabling the most recently changed mod, then launch again.",
            ],
        )
    } else if lower.contains("could not find or load main class")
        || lower.contains("unsatisfiedlinkerror")
        || lower.contains("java.lang.module.resolutionexception")
    {
        (
            "Minecraft couldn't load a required file",
            "The log points to a missing or conflicting game library.",
            NextStep::Repair,
            &[
                "Repair this instance to check and download its game files.",
                "If it still fails, check the loader version and recently added mods.",
            ],
        )
    } else if lower.contains("a fatal error has been detected by the java runtime")
        || lower.contains("exception_access_violation")
    {
        ("Java stopped unexpectedly", "The runtime crashed outside normal game error handling. The log may name the component involved.", NextStep::Logs,
         &["Check the problematic frame in the log for a graphics driver or native library.", "Try a compatible Java runtime and test with overlays disabled."])
    } else {
        ("Minecraft stopped unexpectedly", "This report doesn't identify a clear cause. The exit code alone can't tell us which mod or setting caused it.", NextStep::Logs,
         &["Check the last error in the log below.", "If this started after a change, test that change first. You can copy the report when asking for help."])
    };
    let mut result = advice(title, explanation, next, steps);
    result.evidence = lines
        .into_iter()
        .filter(|line| {
            line.contains("Exception")
                || line.contains("Error")
                || line.contains("Unrecognized VM")
                || line.contains("Could not reserve")
        })
        .filter(|line| !line.starts_with("at "))
        .take(3)
        .collect();
    result
}

fn advice(title: &str, explanation: &str, next: NextStep, steps: &[&str]) -> Advice {
    Advice {
        title: title.into(),
        explanation: explanation.into(),
        steps: steps.iter().map(|s| (*s).into()).collect(),
        evidence: Vec::new(),
        next,
    }
}

fn friendly_mod(name: &str) -> &str {
    match name {
        "fabric-api" => "Fabric API",
        "fabricloader" => "Fabric Loader",
        "minecraft" => "Minecraft",
        _ => name,
    }
}

fn messages(log: &str) -> Vec<String> {
    let mut output = Vec::new();
    for raw in log.lines() {
        let line = raw
            .trim()
            .trim_start_matches("[STDOUT]")
            .trim_start_matches("[STDERR]")
            .trim();
        let line = if let Some((_, body)) = line.split_once("<![CDATA[") {
            body.split("]]>").next().unwrap_or(body)
        } else if line.starts_with('<') {
            continue;
        } else {
            line
        };
        let line = line.trim_start_matches('-').trim();
        if !line.is_empty() {
            let line: String = line.chars().take(650).collect();
            if output.last() != Some(&line) {
                output.push(line);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fabric_solver_xml_explains_the_missing_dependency() {
        let advice = explain("[STDOUT] <log4j:Message><![CDATA[Immediate reason: [HARD_DEP_NO_CANDIDATE craftyai 1.4.0 {depends fabric-api @ [>=0.160.6+26.3]}, ROOT_FORCELOAD_SINGLE craftyai 1.4.0]]]></log4j:Message>", 1);
        assert_eq!(advice.next, NextStep::Mods);
        assert!(advice
            .explanation
            .contains("craftyai 1.4.0 needs Fabric API"));
        assert!(advice.explanation.contains(">=0.160.6+26.3"));
    }
    #[test]
    fn human_requirements_win_over_solver_lines() {
        let advice = explain("HARD_DEP_NO_CANDIDATE x 1 {depends a @ [*]}\n- Mod 'Example' (example) 1 requires version 2 of mod 'Dependency' (dep), which is missing!", 1);
        assert_eq!(advice.evidence.len(), 1);
        assert!(advice.evidence[0].contains("Dependency"));
    }
    #[test]
    fn exit_code_alone_does_not_claim_a_cause() {
        let advice = explain("", 1);
        assert_eq!(advice.next, NextStep::Logs);
        assert!(advice.explanation.contains("doesn't identify"));
    }
    #[test]
    fn distinguishes_insufficient_heap_from_failed_allocation() {
        assert_eq!(
            explain("java.lang.OutOfMemoryError: Java heap space", 1).title,
            "Minecraft ran out of memory"
        );
        assert_eq!(
            explain("Could not reserve enough space for object heap", 1).title,
            "Java couldn't use the memory setting"
        );
        assert_eq!(
            explain("java.lang.UnsupportedClassVersionError", 1).next,
            NextStep::Settings
        );
    }
}
