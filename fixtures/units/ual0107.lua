-- FAF-style unit with two weapons (multi-weapon cadence fixture)
{
    BlueprintId = "ual0107",
    DisplayName = "Aeon T1 Mobile AA",
    UnitId = "ual0107",
    Weapon = {
        {
            BlueprintId = "/projectiles/ual0107/weapon1",
            Damage = 8,
            DamageRadius = 0,
            ProjectilesPerOnFire = 1,
            RateOfFire = 3.0,
            MaxRadius = 32,
            SalvoSize = 1,
            ReloadTime = 0.333,
            TurretCapable = true,
            TargetCategories = { "AIR" }
        },
        {
            BlueprintId = "/projectiles/ual0107/weapon2",
            Damage = 5,
            DamageRadius = 0,
            ProjectilesPerOnFire = 1,
            RateOfFire = 2.0,
            MaxRadius = 28,
            SalvoSize = 2,
            SalvoDelay = 0.1,
            ReloadTime = 0.5,
            TurretCapable = true,
            TargetCategories = { "GROUND" }
        }
    }
}
