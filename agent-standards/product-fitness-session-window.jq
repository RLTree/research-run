def leap_year($year):
  ($year % 400 == 0) or (($year % 4 == 0) and ($year % 100 != 0));

def days_in_month($year; $month):
  if $month == 2 then
    if leap_year($year) then 29 else 28 end
  elif $month == 4 or $month == 6 or $month == 9 or $month == 11 then
    30
  else
    31
  end;

def valid_utc_timestamp:
  if type != "string" then false
  else
    (capture("^(?<year>[0-9]{4})-(?<month>[0-9]{2})-(?<day>[0-9]{2})T(?<hour>[0-9]{2}):(?<minute>[0-9]{2}):(?<second>[0-9]{2})Z$")? // null) as $parts |
    if $parts == null then false
    else
      ($parts | map_values(tonumber)) as $time |
      $time.year >= 1970 and
      $time.month >= 1 and $time.month <= 12 and
      $time.day >= 1 and $time.day <= days_in_month($time.year; $time.month) and
      $time.hour >= 0 and $time.hour <= 23 and
      $time.minute >= 0 and $time.minute <= 59 and
      $time.second >= 0 and $time.second <= 59
    end
  end;

(
  (.session | type == "object") and
  (.session.started_at | valid_utc_timestamp) and
  (.session.completed_at | valid_utc_timestamp) and
  (.session.recovery_seconds | type == "number" and . >= 0) and
  (.session.errors | type == "array") and
  all(.session.errors[];
    (.recovery_seconds | type == "number" and . >= 0))
) and
(
  (.session.started_at | fromdateiso8601) as $started_at |
  (.session.completed_at | fromdateiso8601) as $completed_at |
  $completed_at >= $started_at and
  .session.recovery_seconds == ([.session.errors[].recovery_seconds] | add // 0) and
  .session.recovery_seconds <= ($completed_at - $started_at)
)
