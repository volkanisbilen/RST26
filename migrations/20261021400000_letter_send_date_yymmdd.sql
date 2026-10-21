-- Letters created by the daily quest / GM give-item code stored a unix timestamp in send_date;
-- the client wants yymmdd.
UPDATE letter
SET send_date = (EXTRACT(YEAR FROM to_timestamp(send_date))::int % 100) * 10000
              + EXTRACT(MONTH FROM to_timestamp(send_date))::int * 100
              + EXTRACT(DAY FROM to_timestamp(send_date))::int
WHERE send_date > 991231;
