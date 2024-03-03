import {AccountContributionSettings, AccountContributionSettingsVec, AccountContributionSource, AccountContributionTaxability, AccountSettings, AssetAllocation, FicaJS, JobSettings, PersonSettings, RaiseSettings, RatesSourceHolder, Simulation, TaxSettings} from "../pkg/retirement.js";
import fs from "fs";
import assert from "assert";

const deathCSV = fs.readFileSync("../csv/test_death.csv").toString();
const ratesCSV = fs.readFileSync("../csv/test_rates.csv").toString();

// Due to a bug in the original regression that this is based on, we have to skip the
// first entry in the death rates
let deathRates = Float64Array.from(deathCSV.split("\n").map(a => parseFloat(a)).slice(1));
let rates = ratesCSV.split("\n").slice(1).map(csv => {
    return csv.split(",").map(a => parseFloat(a));
});
let stocks = Float64Array.from(rates.map(a => a[0]));
let bonds = Float64Array.from(rates.map(a => a[1]));
let inflation = Float64Array.from(rates.map(a => a[2]));

/*

let asset_allocation = Rc::new(AssetAllocation::new_linear_glide(1, 0.83, (110 - 27) * 12, 0.0));
let account_settings = AccountSettings::new(50000.0, asset_allocation);
let account_contribution_settings = AccountContributionSettings::new(account_settings, 0.15, AccountContributionSource::Employee, AccountContributionTaxability::PostTax);
let job_settings = JobSettings::new(129000.0 / 12.0, Fica::Exempt, RaiseSettings { amount: 1.05, adjust_for_inflation: true }, vec![account_contribution_settings]);
let death_rates = get_thread_local_rc(&TEST_DEATH_BUILTIN);
let person_settings = PersonSettings::new(27, 0, death_rates);
let brackets = vec![(0.0, 0.1), (10275.0, 0.12), (41775.0, 0.22), (89075.0, 0.24), (170050.0, 0.32), (215950.0, 0.35), (539900.0, 0.37)].iter().map(|b| { TaxBracket { floor: b.0, rate: b.1 } }).collect();
let tax_settings = TaxSettings::new(brackets, true, 12950.0, true );
let simulation = Simulation::new::<rand_pcg::Pcg64Mcg, Tax>(1337, 100, RatesSourceHolder::new_from_custom(Vec::from(TEST_RATES_BUILTIN)), 12, job_settings, person_settings, (65 - 27) * 12, tax_settings);

assert_eq!(simulation.success_rate().num, 48);
assert_eq!(simulation.success_rate().denom, 100);

assert_eq!(simulation.runs[0].lifespan.periods(), 767);
assert_eq!(simulation.runs[0].assets_adequate_periods, 622);
assert_eq!(simulation.runs[0].retirement_accounts[0].balance()[..12], [51248.7292286, 52380.39457286909, 56871.42575448158, 59525.492082032, 61196.13885752394, 61785.05465636826, 65607.00783072409, 67606.33964011342, 67969.00130185773, 71380.60268634508, 73701.0843924699, 75908.8568924566]);
assert_eq!(simulation.runs[0].retirement_accounts[0].balance()[(simulation.runs[0].lifespan.periods()-12)..], [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);

assert_eq!(simulation.runs[1].lifespan.periods(), 691);
assert_eq!(simulation.runs[1].assets_adequate_periods, 691);
assert_eq!(simulation.runs[1].retirement_accounts[0].balance()[..12], [54073.2778065, 56965.67060992199, 59760.360763633245, 63107.109164191774, 68958.71226715308, 67842.21923729929, 74852.24766690681, 72512.28377092176, 74959.52661903139, 76316.76162827399, 77291.47993597148, 80256.03843738187]);
assert_eq!(simulation.runs[1].retirement_accounts[0].balance()[(simulation.runs[1].lifespan.periods()-12)..], [744821.5730118523, 703018.223741604, 663064.1007979073, 611859.271316495, 562289.5586005333, 518130.04116344935, 466121.4553477689, 417050.95723389054, 367524.3422774736, 321111.5655106036, 271257.2857022287, 219811.49559669665]);

*/

let assetAllocation = AssetAllocation.new_linear_glide(1, 0.83, (110 - 27) * 12, 0.0);
let accountSettings = new AccountSettings(50000.0, assetAllocation);
let accountContributionSettings = new AccountContributionSettings(accountSettings, 0.15, AccountContributionSource.Employee, AccountContributionTaxability.PostTax);
let raiseSettings = new RaiseSettings(1.05, true)
let allAccountContributionSettings = new AccountContributionSettingsVec();
allAccountContributionSettings.add(accountContributionSettings);
let jobSettings = new JobSettings(129000 / 12, FicaJS.exempt(), raiseSettings, allAccountContributionSettings);
let personSettings = PersonSettings.new_with_custom_death_rates("John", 27, 0, deathRates);
let taxSettings = new TaxSettings([0, 10275, 41775, 89075, 170050, 215950, 539900], [0.1, 0.12, 0.22, 0.24, 0.32, 0.35, 0.37], true, 12950, true);
let simulation = new Simulation(BigInt(1337), 100, RatesSourceHolder.new_from_custom_split(stocks, bonds, inflation), 12, jobSettings, personSettings, (65 - 27) * 12, taxSettings);

assert.equal(simulation.success_rate().num, 48);
assert.equal(simulation.success_rate().denom, 100);

assert.equal(simulation.lifespan_for_run(0).periods(), 767);
assert.equal(simulation.assets_adequate_periods_for_run(0), 622);
assert.deepEqual(simulation.get_account_balance_for_run(0, 0).slice(0, 12), Float64Array.from([51248.7292286, 52380.39457286909, 56871.42575448158, 59525.492082032, 61196.13885752394, 61785.05465636826, 65607.00783072409, 67606.33964011342, 67969.00130185773, 71380.60268634508, 73701.0843924699, 75908.8568924566]));
assert.deepEqual(simulation.get_account_balance_for_run(0, 0).slice(-12), Float64Array.from([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]));

assert.equal(simulation.lifespan_for_run(1).periods(), 691);
assert.equal(simulation.assets_adequate_periods_for_run(1), 691);
assert.deepEqual(simulation.get_account_balance_for_run(1, 0).slice(0, 12), Float64Array.from([54073.2778065, 56965.67060992199, 59760.360763633245, 63107.109164191774, 68958.71226715308, 67842.21923729929, 74852.24766690681, 72512.28377092176, 74959.52661903139, 76316.76162827399, 77291.47993597148, 80256.03843738187]));
assert.deepEqual(simulation.get_account_balance_for_run(1, 0).slice(-12), Float64Array.from([744821.5730118523, 703018.223741604, 663064.1007979073, 611859.271316495, 562289.5586005333, 518130.04116344935, 466121.4553477689, 417050.95723389054, 367524.3422774736, 321111.5655106036, 271257.2857022287, 219811.49559669665]));
