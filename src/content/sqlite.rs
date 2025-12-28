use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::content::schema::{CONTENT_SCHEMA_VERSION, CONTENT_VERSION, DEFAULT_LOCALE};
use crate::rules::cost::{CostSpec, CostType};
use crate::rules::expression::{
    Constraints, Delivery, ExpressionDef, ExpressionForm, ExpressionText, Scale,
};
use crate::content::repository::{
    ExpressionId, OriginAcquisitionProfile, PersonaExpression, PowerId, PowerInfo, PowerRepository,
    PowerStats,
};
use crate::rules::signature::{SignatureSpec, SignatureType};

pub struct SqlitePowerRepository {
    conn: Connection,
}

impl SqlitePowerRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        validate_content_meta(&conn)?;
        Ok(Self { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn load_expression_defs(
        &self,
    ) -> Result<HashMap<ExpressionId, ExpressionDef>, Box<dyn std::error::Error>> {
        let costs = load_costs(&self.conn)?;
        let signatures = load_signatures(&self.conn)?;

        let mut stmt = self.conn.prepare(
            "SELECT e.expression_id, e.power_id, e.form, e.delivery, e.scale, e.constraints,\
                    t.ui_name, t.tooltip_short\
             FROM power_expression e\
             JOIN power_expression_text t\
               ON t.expression_id = e.expression_id AND t.locale = ?1\
             WHERE e.is_enabled = 1",
        )?;

        let rows = stmt.query_map(params![DEFAULT_LOCALE], |row| {
            let expr_id: String = row.get(0)?;
            let power_id: i64 = row.get(1)?;
            let form: String = row.get(2)?;
            let delivery: String = row.get(3)?;
            let scale: String = row.get(4)?;
            let constraints_raw: String = row.get(5)?;
            let ui_name: String = row.get(6)?;
            let tooltip_short: String = row.get(7)?;
            Ok((
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
            ))
        })?;

        let mut out = HashMap::new();
        for row in rows {
            let (
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
            ) = row?;
            let def = build_expression_def(
                &expr_id,
                power_id,
                &form,
                &delivery,
                &scale,
                &constraints_raw,
                ui_name,
                tooltip_short,
                &costs,
                &signatures,
            )?;
            out.insert(def.id.clone(), def);
        }

        Ok(out)
    }
}

impl PowerRepository for SqlitePowerRepository {
    fn stats(&self) -> Result<PowerStats, Box<dyn std::error::Error>> {
        Ok(PowerStats {
            power_count: count_rows(&self.conn, "Superpower4")?,
            expression_count: count_rows(&self.conn, "power_expression")?,
            acquisition_count: count_rows(&self.conn, "power_acquisition_profile")?,
        })
    }

    fn expression(&self, expr_id: &ExpressionId) -> Result<ExpressionDef, Box<dyn std::error::Error>> {
        let costs = load_costs(&self.conn)?;
        let signatures = load_signatures(&self.conn)?;

        let mut stmt = self.conn.prepare(
            "SELECT e.expression_id, e.power_id, e.form, e.delivery, e.scale, e.constraints,\
                    t.ui_name, t.tooltip_short\
             FROM power_expression e\
             JOIN power_expression_text t\
               ON t.expression_id = e.expression_id AND t.locale = ?1\
             WHERE e.is_enabled = 1 AND e.expression_id = ?2",
        )?;

        let row = stmt.query_row(params![DEFAULT_LOCALE, expr_id.0], |row| {
            let expr_id: String = row.get(0)?;
            let power_id: i64 = row.get(1)?;
            let form: String = row.get(2)?;
            let delivery: String = row.get(3)?;
            let scale: String = row.get(4)?;
            let constraints_raw: String = row.get(5)?;
            let ui_name: String = row.get(6)?;
            let tooltip_short: String = row.get(7)?;
            Ok((
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
            ))
        })?;

        let def = build_expression_def(
            &row.0,
            row.1,
            &row.2,
            &row.3,
            &row.4,
            &row.5,
            row.6,
            row.7,
            &costs,
            &signatures,
        )?;
        Ok(def)
    }

    fn expressions_for_power(
        &self,
        power_id: PowerId,
    ) -> Result<Vec<ExpressionDef>, Box<dyn std::error::Error>> {
        let costs = load_costs(&self.conn)?;
        let signatures = load_signatures(&self.conn)?;

        let mut stmt = self.conn.prepare(
            "SELECT e.expression_id, e.power_id, e.form, e.delivery, e.scale, e.constraints,\
                    t.ui_name, t.tooltip_short\
             FROM power_expression e\
             JOIN power_expression_text t\
               ON t.expression_id = e.expression_id AND t.locale = ?1\
             WHERE e.is_enabled = 1 AND e.power_id = ?2\
             ORDER BY e.expression_id",
        )?;

        let rows = stmt.query_map(params![DEFAULT_LOCALE, power_id.0], |row| {
            let expr_id: String = row.get(0)?;
            let power_id: i64 = row.get(1)?;
            let form: String = row.get(2)?;
            let delivery: String = row.get(3)?;
            let scale: String = row.get(4)?;
            let constraints_raw: String = row.get(5)?;
            let ui_name: String = row.get(6)?;
            let tooltip_short: String = row.get(7)?;
            Ok((
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
            ))
        })?;

        let mut out = Vec::new();
        for row in rows {
            let (
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
            ) = row?;
            out.push(build_expression_def(
                &expr_id,
                power_id,
                &form,
                &delivery,
                &scale,
                &constraints_raw,
                ui_name,
                tooltip_short,
                &costs,
                &signatures,
            )?);
        }

        Ok(out)
    }

    fn power_info(
        &self,
        power_id: PowerId,
    ) -> Result<Option<PowerInfo>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, overview, description FROM Superpower4 WHERE rowid = ?1",
        )?;
        let base = stmt
            .query_row([power_id.0], |row| {
                let name: String = row.get(0)?;
                let overview: Option<String> = row.get(1)?;
                let description: Option<String> = row.get(2)?;
                Ok((name, overview, description))
            })
            .optional()?;

        let Some((name, overview, description)) = base else {
            return Ok(None);
        };

        let text = self
            .conn
            .query_row(
                "SELECT description_short, description_mechanical FROM power_text WHERE power_id = ?1 AND locale = ?2",
                params![power_id.0, DEFAULT_LOCALE],
                |row| {
                    let short: Option<String> = row.get(0)?;
                    let mechanical: Option<String> = row.get(1)?;
                    Ok((short, mechanical))
                },
            )
            .optional()?;

        let (text_short, text_mechanical) = text.unwrap_or((None, None));

        Ok(Some(PowerInfo {
            id: power_id,
            name,
            overview,
            description,
            text_short,
            text_mechanical,
        }))
    }

    fn power_id_by_name(
        &self,
        name: &str,
    ) -> Result<Option<PowerId>, Box<dyn std::error::Error>> {
        let mut stmt =
            self.conn
                .prepare("SELECT rowid FROM Superpower4 WHERE lower(name) = lower(?1) LIMIT 1")?;
        let id = stmt
            .query_row([name], |row| row.get::<_, i64>(0))
            .optional()?;
        Ok(id.map(PowerId))
    }

    fn power_tags(&self, power_id: PowerId) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut stmt =
            self.conn
                .prepare("SELECT tag FROM power_tag WHERE power_id = ?1")?;
        let rows = stmt.query_map([power_id.0], |row| row.get::<_, String>(0))?;
        let mut tags = Vec::new();
        for row in rows {
            let tag = row?.trim().to_string();
            if !tag.is_empty() {
                tags.push(tag);
            }
        }
        Ok(tags)
    }

    fn power_ids_by_tags(
        &self,
        tags_any: &[String],
        tags_all: &[String],
        tags_none: &[String],
    ) -> Result<Vec<PowerId>, Box<dyn std::error::Error>> {
        let normalize = |tag: &str| tag.trim().to_ascii_lowercase();
        let tags_any: HashSet<String> = tags_any.iter().map(|t| normalize(t)).collect();
        let tags_all: HashSet<String> = tags_all.iter().map(|t| normalize(t)).collect();
        let tags_none: HashSet<String> = tags_none.iter().map(|t| normalize(t)).collect();

        let mut stmt = self
            .conn
            .prepare("SELECT power_id, tag FROM power_tag")?;
        let rows = stmt.query_map([], |row| {
            let power_id: i64 = row.get(0)?;
            let tag: String = row.get(1)?;
            Ok((power_id, normalize(&tag)))
        })?;

        let mut map: HashMap<i64, HashSet<String>> = HashMap::new();
        for row in rows {
            let (power_id, tag) = row?;
            if tag.is_empty() {
                continue;
            }
            map.entry(power_id).or_default().insert(tag);
        }

        let mut out = Vec::new();
        for (power_id, tags) in map {
            if !tags_all.is_empty() && !tags_all.is_subset(&tags) {
                continue;
            }
            if !tags_any.is_empty() && tags_any.intersection(&tags).next().is_none() {
                continue;
            }
            if !tags_none.is_empty() && tags_none.intersection(&tags).next().is_some() {
                continue;
            }
            out.push(PowerId(power_id));
        }
        Ok(out)
    }

    fn expressions_for_persona(
        &self,
        persona_id: &str,
    ) -> Result<Vec<PersonaExpression>, Box<dyn std::error::Error>> {
        let costs = load_costs(&self.conn)?;
        let signatures = load_signatures(&self.conn)?;

        let mut stmt = self.conn.prepare(
            "SELECT pe.expression_id, pe.power_id, pe.form, pe.delivery, pe.scale, pe.constraints,\
                    pet.ui_name, pet.tooltip_short,\
                    pp.persona_id, pp.mastery_level, pp.modifiers, pp.is_unlocked\
             FROM persona_power pp\
             JOIN power_expression pe\
               ON pe.expression_id = pp.expression_id AND pe.is_enabled = 1\
             JOIN power_expression_text pet\
               ON pet.expression_id = pe.expression_id AND pet.locale = ?1\
             WHERE pp.persona_id = ?2 AND pp.is_unlocked = 1",
        )?;

        let rows = stmt.query_map(params![DEFAULT_LOCALE, persona_id], |row| {
            let expr_id: String = row.get(0)?;
            let power_id: i64 = row.get(1)?;
            let form: String = row.get(2)?;
            let delivery: String = row.get(3)?;
            let scale: String = row.get(4)?;
            let constraints_raw: String = row.get(5)?;
            let ui_name: String = row.get(6)?;
            let tooltip_short: String = row.get(7)?;
            let persona_id: String = row.get(8)?;
            let mastery_level: i64 = row.get(9)?;
            let modifiers_raw: String = row.get(10)?;
            let is_unlocked: i64 = row.get(11)?;
            Ok((
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
                persona_id,
                mastery_level,
                modifiers_raw,
                is_unlocked,
            ))
        })?;

        let mut out = Vec::new();
        for row in rows {
            let (
                expr_id,
                power_id,
                form,
                delivery,
                scale,
                constraints_raw,
                ui_name,
                tooltip_short,
                persona_id,
                mastery_level,
                modifiers_raw,
                is_unlocked,
            ) = row?;
            let expression = build_expression_def(
                &expr_id,
                power_id,
                &form,
                &delivery,
                &scale,
                &constraints_raw,
                ui_name,
                tooltip_short,
                &costs,
                &signatures,
            )?;
            let modifiers: Value = serde_json::from_str(&modifiers_raw)?;
            out.push(PersonaExpression {
                persona_id,
                mastery_level,
                modifiers,
                is_unlocked: is_unlocked != 0,
                expression,
            });
        }

        Ok(out)
    }

    fn acquisition_profiles_for_origin(
        &self,
        origin_class: &str,
        origin_subtype: &str,
    ) -> Result<Vec<OriginAcquisitionProfile>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT acq_id, power_id, rarity_weight\
             FROM power_acquisition_profile\
             WHERE is_enabled = 1\
               AND origin_class = ?1\
               AND (origin_subtype = ?2 OR origin_subtype IS NULL OR origin_subtype = '')",
        )?;

        let rows = stmt.query_map(params![origin_class, origin_subtype], |row| {
            Ok(OriginAcquisitionProfile {
                acq_id: row.get(0)?,
                power_id: PowerId(row.get(1)?),
                rarity_weight: row.get(2)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }

        Ok(out)
    }
}

fn build_expression_def(
    expr_id: &str,
    power_id: i64,
    form: &str,
    delivery: &str,
    scale: &str,
    constraints_raw: &str,
    ui_name: String,
    tooltip_short: String,
    costs: &HashMap<String, Vec<CostSpec>>,
    signatures: &HashMap<String, Vec<SignatureSpec>>,
) -> Result<ExpressionDef, Box<dyn std::error::Error>> {
    let constraints_json: Value = serde_json::from_str(constraints_raw)?;
    let constraints = Constraints::from_json(&constraints_json);
    let form = ExpressionForm::from_str(form)?;
    let delivery = Delivery::from_str(delivery)?;
    let scale = Scale::from_str(scale)?;
    let expr_id = ExpressionId(expr_id.to_string());

    let def = ExpressionDef {
        id: expr_id.clone(),
        power_id: PowerId(power_id),
        form,
        delivery,
        scale,
        constraints,
        text: ExpressionText {
            ui_name,
            tooltip_short,
        },
        costs: costs.get(&expr_id.0).cloned().unwrap_or_default(),
        signatures: signatures
            .get(&expr_id.0)
            .cloned()
            .unwrap_or_default(),
    };
    def.validate_defaults()?;
    Ok(def)
}

fn load_costs(conn: &Connection) -> Result<HashMap<String, Vec<CostSpec>>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT expression_id, cost_type, value, risk_type, risk_chance\
         FROM power_expression_cost",
    )?;

    let rows = stmt.query_map([], |row| {
        let expression_id: String = row.get(0)?;
        let cost_type: String = row.get(1)?;
        let value: Option<i64> = row.get(2)?;
        let risk_type: Option<String> = row.get(3)?;
        let risk_chance: Option<f64> = row.get(4)?;
        Ok((expression_id, cost_type, value, risk_type, risk_chance))
    })?;

    let mut out: HashMap<String, Vec<CostSpec>> = HashMap::new();
    for row in rows {
        let (expr_id, cost_type, value, risk_type, risk_chance) = row?;
        let cost_type = CostType::from_str(&cost_type)?;
        out.entry(expr_id).or_default().push(CostSpec {
            cost_type,
            value,
            risk_type,
            risk_chance,
        });
    }

    Ok(out)
}

fn load_signatures(
    conn: &Connection,
) -> Result<HashMap<String, Vec<SignatureSpec>>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(
        "SELECT expression_id, signature_type, strength, persistence_turns\
         FROM power_expression_signature",
    )?;

    let rows = stmt.query_map([], |row| {
        let expression_id: String = row.get(0)?;
        let signature_type: String = row.get(1)?;
        let strength: i64 = row.get(2)?;
        let persistence_turns: i64 = row.get(3)?;
        Ok((expression_id, signature_type, strength, persistence_turns))
    })?;

    let mut out: HashMap<String, Vec<SignatureSpec>> = HashMap::new();
    for row in rows {
        let (expr_id, signature_type, strength, persistence_turns) = row?;
        let signature_type = SignatureType::from_str(&signature_type)?;
        out.entry(expr_id).or_default().push(SignatureSpec {
            signature_type,
            strength,
            persistence_turns,
        });
    }

    Ok(out)
}

fn count_rows(conn: &Connection, table: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let sql = format!("SELECT COUNT(*) FROM {}", table);
    let count = conn.query_row(&sql, [], |row| row.get::<_, i64>(0))?;
    Ok(count)
}

fn validate_content_meta(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let table = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='content_meta'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if table.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "content_meta table missing (rebuild content_v1.db with tools/export_content_db.py)",
        )
        .into());
    }

    let meta = conn
        .query_row(
            "SELECT schema_version, content_version FROM content_meta WHERE id = 1",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    let Some((schema_version, content_version)) = meta else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "content_meta missing row id=1",
        )
        .into());
    };

    if schema_version != CONTENT_SCHEMA_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "content_meta schema_version {} != expected {}",
                schema_version, CONTENT_SCHEMA_VERSION
            ),
        )
        .into());
    }
    if content_version != CONTENT_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "content_meta content_version {} != expected {}",
                content_version, CONTENT_VERSION
            ),
        )
        .into());
    }

    Ok(())
}
