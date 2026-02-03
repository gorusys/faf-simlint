-- FAF-style unit blueprint fixture (UEF T1 Tank)
{
    BlueprintId = "uel0101",
    DisplayName = "UEF T1 Tank",
    UnitId = "uel0101",
    Weapon = {
        {
            BlueprintId = "/projectiles/uel0101/uel0101_weapon1",
            Damage = 10,
            DamageRadius = 0,
            ProjectilesPerOnFire = 1,
            RateOfFire = 2.0,
            MaxRadius = 20,
            MuzzleVelocity = 25,
            SalvoSize = 1,
            ReloadTime = 0,
            TurretCapable = true,
            TargetCategories = { "GROUND", "STRUCTURE" }
        }
    }
}
