-- Correct CREATE_NEW_CHAR_VALUE mapping to match MSSQL LOAD_NEW_CHAR_VALUE.
UPDATE create_new_char_value
SET level = CASE job_type
    WHEN 0 THEN 80
    WHEN 1 THEN 59
    WHEN 2 THEN 69
    WHEN 3 THEN 1
    WHEN 4 THEN 83
    ELSE level END,
    free_points = CASE WHEN job_type = 0 THEN 277 WHEN job_type = 4 THEN 292 ELSE 0 END,
    skill_point_free = CASE WHEN job_type = 0 THEN 142 WHEN job_type = 4 THEN 148 ELSE 0 END,
    gold = CASE WHEN job_type IN (0, 4) THEN 3000000 ELSE 50000 END
WHERE job_type BETWEEN 0 AND 4;
